use crate::cli::progress::StepContext;
use crate::config::sram::{ControlMode, SramConfig, SramParams};
use crate::layout::sram::draw_sram;
use crate::paths::{out_bin, out_gds, out_sram, out_verilog};
use crate::plan::extract::ExtractionResult;
use crate::schematic::sram::sram;
use crate::schematic::{generate_netlist, save_modules};
use crate::verilog::save_1rw_verilog;
use crate::{clog2, Result};
use anyhow::{bail, Context};
use pdkprims::tech::sky130;
use std::collections::HashSet;
use std::path::Path;

pub mod extract;

/// A concrete plan for an SRAM.
///
/// Has a 1-1 mapping with a schematic.
pub struct SramPlan {
    pub sram_params: SramParams,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum TaskKey {
    GeneratePlan,
    GenerateNetlist,
    GenerateLayout,
    GenerateVerilog,
    #[cfg(feature = "commercial")]
    GenerateLef,
    #[cfg(feature = "commercial")]
    RunDrc,
    #[cfg(feature = "commercial")]
    RunLvs,
    #[cfg(feature = "commercial")]
    RunPex,
    #[cfg(feature = "commercial")]
    GenerateLib,
    #[cfg(feature = "commercial")]
    RunSpectre,
    #[cfg(feature = "commercial")]
    All,
}

pub struct ExecutePlanParams<'a> {
    pub work_dir: &'a Path,
    pub plan: &'a SramPlan,
    pub tasks: &'a HashSet<TaskKey>,
    pub ctx: Option<&'a mut StepContext>,
    #[cfg(feature = "commercial")]
    pub pex_level: Option<calibre::pex::PexLevel>,
}

pub fn generate_plan(
    _extraction_result: ExtractionResult,
    config: &SramConfig,
) -> Result<SramPlan> {
    let &SramConfig {
        num_words,
        data_width,
        mux_ratio,
        write_size,
        control,
        ..
    } = config;

    if control != ControlMode::Simple && control != ControlMode::ReplicaV1 {
        bail!(
            "Only `ControlMode::Simple` and `ControlMode::ReplicaV1` are supported at the moment"
        );
    }
    if data_width % write_size != 0 {
        bail!("Data width must be a multiple of write size");
    }

    let name = out_sram(config);
    let rows = (num_words / mux_ratio) as usize;
    let cols = (data_width * mux_ratio) as usize;
    let row_bits = clog2(rows);
    let col_bits = clog2(cols);
    let col_select_bits = clog2(mux_ratio as usize);
    let wmask_width = (data_width / write_size) as usize;
    let mux_ratio = mux_ratio as usize;
    let num_words = num_words as usize;
    let data_width = data_width as usize;
    let addr_width = clog2(num_words);

    Ok(SramPlan {
        sram_params: SramParams {
            name,
            wmask_width,
            row_bits,
            col_bits,
            col_select_bits,
            rows,
            cols,
            mux_ratio,
            num_words,
            data_width,
            addr_width,
            control,
        },
    })
}

macro_rules! try_finish_task {
    ( $ctx:expr, $task:expr ) => {
        if let Some(ctx) = $ctx.as_mut() {
            ctx.finish($task);
        }
    };
}

#[cfg(feature = "commercial")]
macro_rules! try_execute_task {
    ( $tasks:expr, $task:expr, $body:expr, $ctx:expr) => {
        if $tasks.contains(&$task) || $tasks.contains(&TaskKey::All) {
            $body;
            try_finish_task!($ctx, $task);
        }
    };
}

pub fn execute_plan(params: ExecutePlanParams) -> Result<()> {
    let ExecutePlanParams {
        work_dir,
        plan,
        mut ctx,
        ..
    } = params;

    std::fs::create_dir_all(work_dir)?;

    let modules = sram(&plan.sram_params);

    let name = &plan.sram_params.name;

    let bin_path = out_bin(work_dir, name);
    save_modules(&bin_path, name, modules).with_context(|| "Error saving netlist binaries")?;

    generate_netlist(&bin_path, work_dir)
        .with_context(|| "Error converting netlists to SPICE format")?;

    try_finish_task!(ctx, TaskKey::GenerateNetlist);

    let mut lib = sky130::pdk_lib(name)?;
    draw_sram(&mut lib, &plan.sram_params).with_context(|| "Error generating SRAM layout")?;

    let gds_path = out_gds(work_dir, name);
    lib.save_gds(&gds_path)
        .with_context(|| "Error saving SRAM GDS")?;

    try_finish_task!(ctx, TaskKey::GenerateLayout);

    let verilog_path = out_verilog(work_dir, name);
    save_1rw_verilog(&verilog_path, &plan.sram_params)
        .with_context(|| "Error generating or saving Verilog model")?;

    try_finish_task!(ctx, TaskKey::GenerateVerilog);

    #[cfg(feature = "commercial")]
    {
        try_execute_task!(
            params.tasks,
            TaskKey::GenerateLef,
            crate::abs::run_sram_abstract(
                work_dir,
                name,
                crate::paths::out_lef(work_dir, name),
                &gds_path,
                &verilog_path
            )?,
            ctx
        );

        try_execute_task!(
            params.tasks,
            TaskKey::RunDrc,
            crate::verification::calibre::run_sram_drc(work_dir, name)?,
            ctx
        );
        try_execute_task!(
            params.tasks,
            TaskKey::RunLvs,
            crate::verification::calibre::run_sram_lvs(work_dir, name, plan.sram_params.control)?,
            ctx
        );

        if params.pex_level.is_none() && params.tasks.contains(&TaskKey::RunPex) {
            bail!("Must specify a PEX level when running PEX");
        }
        let pex_netlist_path = params
            .pex_level
            .map(|pex_level| crate::paths::out_pex(work_dir, name, pex_level));

        try_execute_task!(
            params.tasks,
            TaskKey::RunPex,
            crate::verification::calibre::run_sram_pex(
                work_dir,
                pex_netlist_path.as_ref().unwrap(),
                name,
                plan.sram_params.control,
                params.pex_level.unwrap(),
            )?,
            ctx
        );

        try_execute_task!(
            params.tasks,
            TaskKey::RunSpectre,
            crate::verification::spectre::run_sram_spectre(&plan.sram_params, work_dir, name)?,
            ctx
        );

        try_execute_task!(
            params.tasks,
            TaskKey::GenerateLib,
            {
                use crate::verification::{source_files, VerificationTask};
                use liberate_mx::LibParams;

                let (source_paths, lib_file) = if let Some(pex_netlist_path) = pex_netlist_path {
                    if !pex_netlist_path.exists() {
                        bail!("PEX netlist not found at path `{:?}`", pex_netlist_path);
                    }
                    (
                        vec![pex_netlist_path],
                        work_dir.join(format!(
                            "{}_tt_025C_1v80.{}.lib",
                            params.plan.sram_params.name,
                            params.pex_level.unwrap()
                        )),
                    )
                } else {
                    (
                        source_files(
                            work_dir,
                            &plan.sram_params.name,
                            VerificationTask::SpectreSim,
                            plan.sram_params.control,
                        ),
                        work_dir.join(format!(
                            "{}_tt_025C_1v80.schematic.lib",
                            params.plan.sram_params.name
                        )),
                    )
                };

                let params = LibParams::builder()
                    .work_dir(work_dir.join("lib"))
                    .output_file(lib_file)
                    .corner("tt")
                    .cell_name(&plan.sram_params.name)
                    .num_words(plan.sram_params.num_words)
                    .data_width(plan.sram_params.data_width)
                    .addr_width(plan.sram_params.addr_width)
                    .wmask_width(plan.sram_params.wmask_width)
                    .mux_ratio(plan.sram_params.mux_ratio)
                    .source_paths(source_paths)
                    .build()?;

                crate::liberate::generate_sram_lib(&params)?;
            },
            ctx
        );
    }
    Ok(())
}
