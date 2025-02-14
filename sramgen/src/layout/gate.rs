use crate::config::gate::{AndParams, GateParams, Size};
use crate::layout::Result;
use layout21::raw::align::AlignRect;
use layout21::raw::geom::Dir;
use layout21::raw::{
    Abstract, AbstractPort, BoundBoxTrait, Cell, Instance, Layout, Point, Rect, Shape,
};
use layout21::utils::Ptr;
use pdkprims::mos::{Intent, MosDevice, MosParams, MosType};
use pdkprims::PdkLib;

use super::draw_rect;
use super::route::Router;

pub fn draw_and2(lib: &mut PdkLib, params: &AndParams) -> Result<Ptr<Cell>> {
    let nand = draw_nand2(lib, &params.nand)?;
    let inv = draw_inv(lib, &params.inv)?;

    let mut layout = Layout::new(&params.name);
    let mut abs = Abstract::new(&params.name);

    let nand = Instance::new("nand2", nand);
    let mut inv = Instance::new("inv", inv);

    let nand_bbox = nand.bbox();

    inv.align_centers_vertically_gridded(nand_bbox, lib.pdk.grid());
    inv.align_to_the_right_of(nand_bbox, 1_000);

    let mut router = Router::new(format!("{}_routing", &params.name), lib.pdk.clone());
    let m0 = lib.pdk.metal(0);

    let src = nand.port("y").largest_rect(m0).unwrap();
    let dst = inv.port("din").largest_rect(m0).unwrap();
    let mut trace = router.trace(src, 0);
    trace.s_bend(dst, Dir::Horiz);

    // Add ports
    abs.add_port(nand.port("a"));
    abs.add_port(nand.port("b"));
    abs.add_port(nand.port("vss").named("vss0"));
    abs.add_port(nand.port("vpb").named("vpb0"));
    abs.add_port(nand.port("vdd").named("vdd0"));
    abs.add_port(nand.port("nsdm").named("nsdm0"));
    abs.add_port(nand.port("psdm").named("psdm0"));
    abs.add_port(inv.port("vss").named("vss1"));
    abs.add_port(inv.port("vdd").named("vdd1"));
    abs.add_port(inv.port("din_b").named("Y"));
    abs.add_port(inv.port("vpb").named("vpb1"));
    abs.add_port(inv.port("nsdm").named("nsdm1"));
    abs.add_port(inv.port("psdm").named("psdm1"));

    layout.add_inst(nand);
    layout.add_inst(inv);
    layout.add_inst(router.finish());

    let cell = Cell {
        name: params.name.clone(),
        layout: Some(layout),
        abs: Some(abs),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_and3(lib: &mut PdkLib, params: &AndParams) -> Result<Ptr<Cell>> {
    let nand = draw_nand3(lib, &params.nand)?;
    let inv = draw_inv(lib, &params.inv)?;

    let mut layout = Layout::new(&params.name);
    let mut abs = Abstract::new(&params.name);

    let nand = Instance::new("nand3", nand);
    let mut inv = Instance::new("inv", inv);

    let nand_bbox = nand.bbox();

    inv.align_centers_vertically_gridded(nand_bbox, lib.pdk.grid());
    inv.align_to_the_right_of(nand_bbox, 1_000);

    let mut router = Router::new(format!("{}_routing", &params.name), lib.pdk.clone());
    let m0 = lib.pdk.metal(0);

    let src = nand.port("y").largest_rect(m0).unwrap();
    let dst = inv.port("din").largest_rect(m0).unwrap();
    let mut trace = router.trace(src, 0);
    trace.s_bend(dst, Dir::Horiz);

    // Add ports
    abs.add_port(nand.port("a"));
    abs.add_port(nand.port("b"));
    abs.add_port(nand.port("c"));
    abs.add_port(nand.port("vss").named("vss0"));
    abs.add_port(nand.port("vdd0").named("vdd0"));
    abs.add_port(nand.port("vdd1").named("vdd1"));
    abs.add_port(nand.port("vpb").named("vpb0"));
    abs.add_port(nand.port("nsdm").named("nsdm0"));
    abs.add_port(nand.port("psdm").named("psdm0"));
    abs.add_port(inv.port("vss").named("vss1"));
    abs.add_port(inv.port("vdd").named("vdd2"));
    abs.add_port(inv.port("din_b").named("Y"));
    abs.add_port(inv.port("vpb").named("vpb1"));
    abs.add_port(inv.port("nsdm").named("nsdm1"));
    abs.add_port(inv.port("psdm").named("psdm1"));

    layout.add_inst(nand);
    layout.add_inst(inv);
    layout.add_inst(router.finish());

    let cell = Cell {
        name: params.name.clone(),
        layout: Some(layout),
        abs: Some(abs),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_nand2(lib: &mut PdkLib, args: &GateParams) -> Result<Ptr<Cell>> {
    let mut layout = Layout::new(&args.name);
    let mut abs = Abstract::new(&args.name);

    let mut params = MosParams::new();
    params
        .dnw(false)
        .direction(Dir::Horiz)
        .add_device(MosDevice {
            mos_type: MosType::Nmos,
            width: args.size.nmos_width,
            length: args.length,
            fingers: 2,
            intent: Intent::Svt,
            skip_sd_metal: vec![1],
        })
        .add_device(MosDevice {
            mos_type: MosType::Pmos,
            width: args.size.pmos_width,
            length: args.length,
            fingers: 2,
            intent: Intent::Svt,
            skip_sd_metal: vec![],
        });
    let ptx = lib.draw_mos(params)?;

    let inst = Instance::new("mos", ptx.cell.clone());
    layout.insts.push(inst.clone());

    let tc = lib.pdk.config.read().unwrap();

    let ndrain = ptx.sd_pin(0, 2).unwrap();
    let pdrain1 = ptx.sd_pin(1, 2).unwrap();
    let pdrain0 = ptx.sd_pin(1, 0).unwrap();

    let xlim = pdrain0.p0.x - tc.layer("li").space;

    let cx = (ndrain.p1.x + pdrain1.p0.x) / 2;

    let (mut xmin, mut xmax) = lib.pdk.gridded_center_span(cx, tc.layer("li").width);

    if xmax > xlim {
        let xshift = xmax - xlim;
        xmin -= xshift;
        xmax -= xshift;
    }

    let mut port_vss = ptx.sd_port(0, 0).unwrap();
    port_vss.set_net("vss");
    abs.add_port(port_vss);

    let mut port_vdd = ptx.sd_port(1, 1).unwrap();
    port_vdd.set_net("vdd");
    abs.add_port(port_vdd);

    let mut port_a = ptx.gate_port(0).unwrap();
    port_a.set_net("a");
    abs.add_port(port_a);

    let mut port_b = ptx.gate_port(1).unwrap();
    port_b.set_net("b");
    abs.add_port(port_b);

    let mut port_y = AbstractPort::new("y");

    let rects = [
        Rect {
            p0: Point::new(ndrain.p0.x, ndrain.p0.y),
            p1: Point::new(pdrain1.p1.x, pdrain1.p1.y),
        },
        Rect {
            p0: Point::new(xmin, pdrain0.p0.y),
            p1: Point::new(xmax, pdrain1.p1.y),
        },
        Rect {
            p0: Point::new(xmin, pdrain0.p0.y),
            p1: Point::new(pdrain0.p1.x, pdrain0.p1.y),
        },
    ];

    for r in rects {
        layout.elems.push(draw_rect(r, ptx.sd_metal));
        port_y.add_shape(ptx.sd_metal, Shape::Rect(r));
    }

    abs.add_port(ptx.merged_vpb_port(1));
    abs.add_port(inst.port("nsdm_0").named("nsdm"));
    abs.add_port(inst.port("psdm_1").named("psdm"));

    abs.add_port(port_y);

    let cell = Cell {
        name: args.name.clone(),
        abs: Some(abs),
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_nor2(lib: &mut PdkLib, args: &GateParams) -> Result<Ptr<Cell>> {
    let mut layout = Layout::new(&args.name);
    let mut abs = Abstract::new(&args.name);

    let mut params = MosParams::new();
    params
        .dnw(false)
        .direction(Dir::Horiz)
        .add_device(MosDevice {
            mos_type: MosType::Nmos,
            width: args.size.nmos_width,
            length: args.length,
            fingers: 2,
            intent: Intent::Svt,
            skip_sd_metal: vec![],
        })
        .add_device(MosDevice {
            mos_type: MosType::Pmos,
            width: args.size.pmos_width,
            length: args.length,
            fingers: 2,
            intent: Intent::Svt,
            skip_sd_metal: vec![1],
        });
    let ptx = lib.draw_mos(params)?;

    layout.insts.push(Instance::new("mos", ptx.cell.clone()));

    let tc = lib.pdk.config.read().unwrap();

    let ndrain1 = ptx.sd_pin(0, 2).unwrap();
    let ndrain2 = ptx.sd_pin(0, 0).unwrap();
    let pdrain = ptx.sd_pin(1, 0).unwrap();

    let xlim = pdrain.p0.x - tc.layer("li").space;

    let cx = (ndrain1.p1.x + pdrain.p0.x) / 2;

    let (mut xmin, mut xmax) = lib.pdk.gridded_center_span(cx, tc.layer("li").width);

    if xmax > xlim {
        let xshift = xmax - xlim;
        xmin -= xshift;
        xmax -= xshift;
    }

    abs.add_port(ptx.sd_port(0, 0).unwrap().named("vss"));
    abs.add_port(ptx.sd_port(1, 2).unwrap().named("vdd"));
    abs.add_port(ptx.gate_port(0).unwrap().named("a"));
    abs.add_port(ptx.gate_port(1).unwrap().named("b"));

    let mut port_y = AbstractPort::new("y");

    let rects = [
        Rect {
            p0: Point::new(ndrain2.p0.x, ndrain2.p0.y),
            p1: Point::new(pdrain.p1.x, pdrain.p1.y),
        },
        Rect {
            p0: Point::new(xmin, pdrain.p0.y),
            p1: Point::new(xmax, ndrain1.p1.y),
        },
        Rect {
            p0: Point::new(ndrain1.p0.x, ndrain1.p0.y),
            p1: Point::new(xmax, ndrain1.p1.y),
        },
    ];

    for r in rects {
        layout.elems.push(draw_rect(r, ptx.sd_metal));
        port_y.add_shape(ptx.sd_metal, Shape::Rect(r));
    }

    abs.add_port(port_y);

    let cell = Cell {
        name: args.name.clone(),
        abs: Some(abs),
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_nand3(lib: &mut PdkLib, args: &GateParams) -> Result<Ptr<Cell>> {
    let mut layout = Layout::new(&args.name);
    let mut abs = Abstract::new(&args.name);

    let mut params = MosParams::new();
    params
        .dnw(false)
        .direction(Dir::Horiz)
        .add_device(MosDevice {
            mos_type: MosType::Nmos,
            width: args.size.nmos_width,
            length: args.length,
            fingers: 3,
            intent: Intent::Svt,
            skip_sd_metal: vec![1, 2],
        })
        .add_device(MosDevice {
            mos_type: MosType::Pmos,
            width: args.size.pmos_width,
            length: args.length,
            fingers: 3,
            intent: Intent::Svt,
            skip_sd_metal: Vec::new(),
        });
    let ptx = lib.draw_mos(params)?;

    let inst = Instance::new("mos", ptx.cell.clone());

    layout.insts.push(inst.clone());

    let tc = lib.pdk.config.read().unwrap();

    let ny = ptx.sd_pin(0, 3).unwrap();
    let py1 = ptx.sd_pin(1, 0).unwrap();
    let py2 = ptx.sd_pin(1, 2).unwrap();

    // Check that metal 0 spacing restrictions are satisfied
    // (In SKY130, metal 0 is local interconnect)
    let m0 = lib.pdk.metal_name(0);
    let xlim_h = py1.p0.x - tc.layer(m0).space;
    let xlim_l = ny.p1.x + tc.layer(m0).space;
    let width = tc.layer(m0).width;

    assert!(
        xlim_h - xlim_l >= width,
        "Not enough space to route NAND3 gate"
    );

    let cx = (xlim_l + xlim_h) / 2;

    let (xmin, xmax) = lib.pdk.gridded_center_span(cx, tc.layer("li").width);

    assert!(xmax <= xlim_h);
    assert!(xmin >= xlim_l);

    abs.add_port(ptx.sd_port(0, 0).unwrap().named("vss"));
    abs.add_port(ptx.sd_port(1, 1).unwrap().named("vdd0"));
    abs.add_port(ptx.sd_port(1, 3).unwrap().named("vdd1"));
    abs.add_port(ptx.gate_port(0).unwrap().named("c"));
    abs.add_port(ptx.gate_port(1).unwrap().named("b"));
    abs.add_port(ptx.gate_port(2).unwrap().named("a"));
    abs.add_port(ptx.merged_vpb_port(1));
    abs.add_port(inst.port("nsdm_0").named("nsdm"));
    abs.add_port(inst.port("psdm_1").named("psdm"));

    let mut port_y = AbstractPort::new("y");

    let rects = [
        Rect::new(Point::new(ny.p0.x, ny.p0.y), Point::new(xmax, ny.p1.y)),
        Rect::new(Point::new(xmin, py1.p0.y), Point::new(xmax, ny.p1.y)),
        Rect::new(Point::new(xmin, py1.p0.y), Point::new(py1.p1.x, py1.p1.y)),
        Rect::new(Point::new(xmin, py2.p0.y), Point::new(py2.p1.x, py2.p1.y)),
    ];

    for r in rects {
        layout.elems.push(draw_rect(r, ptx.sd_metal));
    }

    port_y.add_shape(ptx.sd_metal, Shape::Rect(rects[3]));

    abs.add_port(port_y);

    let cell = Cell {
        name: args.name.clone(),
        abs: Some(abs),
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}

pub fn draw_inv_dec(lib: &mut PdkLib, name: impl Into<String>) -> Result<Ptr<Cell>> {
    draw_inv(
        lib,
        &GateParams {
            name: name.into(),
            size: Size {
                nmos_width: 1_600,
                pmos_width: 2_400,
            },
            length: 150,
        },
    )
}

pub fn draw_inv(lib: &mut PdkLib, args: &GateParams) -> Result<Ptr<Cell>> {
    let mut layout = Layout::new(&args.name);
    let mut abs = Abstract::new(&args.name);

    let mut params = MosParams::new();
    params
        .dnw(false)
        .direction(Dir::Horiz)
        .add_device(MosDevice {
            mos_type: MosType::Nmos,
            width: args.size.nmos_width,
            length: args.length,
            fingers: 1,
            intent: Intent::Svt,
            skip_sd_metal: vec![],
        })
        .add_device(MosDevice {
            mos_type: MosType::Pmos,
            width: args.size.pmos_width,
            length: args.length,
            fingers: 1,
            intent: Intent::Svt,
            skip_sd_metal: vec![],
        });
    let ptx = lib.draw_mos(params)?;

    let inst = Instance::new("mos", ptx.cell.clone());

    layout.insts.push(inst.clone());

    let mut port_vss = ptx.sd_port(0, 0).unwrap();
    port_vss.set_net("vss");
    abs.add_port(port_vss);

    let mut port_vdd = ptx.sd_port(1, 0).unwrap();
    port_vdd.set_net("vdd");
    abs.add_port(port_vdd);

    let dout_n = ptx.sd_pin(0, 1).unwrap();
    let dout_p = ptx.sd_pin(1, 1).unwrap();

    let rect = Rect {
        p0: Point::new(dout_n.p0.x, dout_n.p0.y),
        p1: Point::new(dout_p.p1.x, dout_p.p1.y),
    };

    let mut port_din_b = AbstractPort::new("din_b");
    port_din_b.add_shape(ptx.sd_metal, Shape::Rect(rect));
    abs.add_port(port_din_b);

    let mut port_din = ptx.gate_port(0).unwrap();
    port_din.set_net("din");
    abs.add_port(port_din);

    abs.add_port(ptx.merged_vpb_port(1));
    abs.add_port(inst.port("nsdm_0").named("nsdm"));
    abs.add_port(inst.port("psdm_1").named("psdm"));

    layout.elems.push(draw_rect(rect, ptx.sd_metal));

    let cell = Cell {
        name: args.name.clone(),
        abs: Some(abs),
        layout: Some(layout),
    };

    let ptr = Ptr::new(cell);
    lib.lib.cells.push(ptr.clone());

    Ok(ptr)
}
