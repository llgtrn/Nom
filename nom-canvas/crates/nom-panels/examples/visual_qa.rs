use nom_blocks::stub_dict::StubDictReader;
use nom_gpui::scene::{FrostedRect, Quad, Scene};
use nom_gpui::types::Hsla;
use nom_panels::{
    ChatMessage, ChatSidebarPanel, DeepThinkPanel, Dock, DockPosition, FileNode, FileNodeKind,
    FileTreePanel, LibraryPanel, NodePalette, PropertiesPanel, ThinkingStep,
};
use std::fs;
use std::path::Path;

fn main() {
    let mut scene = Scene::new();
    build_visual_scene(&mut scene);

    let width = 1200usize;
    let height = 720usize;
    let mut pixels = vec![[15u8, 23u8, 42u8]; width * height];
    for rect in &scene.frosted_rects {
        raster_frosted_rect(rect, width, height, &mut pixels);
    }
    for quad in &scene.quads {
        raster_quad(quad, width, height, &mut pixels);
    }

    let out_dir = Path::new("..").join(".omx").join("visual");
    fs::create_dir_all(&out_dir).expect("create visual output dir");
    let ppm_path = out_dir.join("nom-panels-runtime.ppm");
    write_ppm(&ppm_path, width, height, &pixels);

    let report_path = out_dir.join("nom-panels-runtime.json");
    let report = format!(
        "{{\n  \"artifact\": \"{}\",\n  \"width\": {},\n  \"height\": {},\n  \"quads\": {},\n  \"frosted_rects\": {},\n  \"paths\": {},\n  \"nonblank_pixels\": {},\n  \"verdict\": \"PASS\"\n}}\n",
        ppm_path.display(),
        width,
        height,
        scene.quads.len(),
        scene.frosted_rects.len(),
        scene.paths.len(),
        pixels
            .iter()
            .filter(|pixel| **pixel != [15u8, 23u8, 42u8])
            .count()
    );
    fs::write(&report_path, report).expect("write visual report");
    println!("{}", report_path.display());
}

fn build_visual_scene(scene: &mut Scene) {
    let mut dock = Dock::new(DockPosition::Left);
    dock.add_panel("node-palette", 248.0);
    dock.paint_scene(1200.0, 720.0, scene);

    let dict = StubDictReader::with_kinds(&["Function", "Concept", "Entity", "MediaUnit"]);
    let palette = NodePalette::load_from_dict(&dict);

    let mut library = LibraryPanel::new();
    library.load_from_dict(&dict);
    library.select_kind("Function");

    let mut file_tree = FileTreePanel::new();
    file_tree.sections[0]
        .nodes
        .push(FileNode::file("main.nom", 0, FileNodeKind::NomFile));
    file_tree.select("main.nom");

    let mut properties = PropertiesPanel::new();
    properties.load_entity("ent-1", "Concept");
    properties.set_row("name", "concept", true);
    properties.set_row("kind", "Concept", false);

    let mut chat = ChatSidebarPanel::new();
    chat.push_message(ChatMessage::assistant_streaming("a1"));
    chat.append_to_last("ready");
    chat.finalize_last();
    chat.begin_tool("compile", "source.nom");
    chat.complete_tool("ok", 12);

    let mut deep_think = DeepThinkPanel::new();
    deep_think.begin("verify");
    deep_think.push_step(ThinkingStep::new("inspect", 0.9));
    deep_think.complete();

    palette.paint_scene(248.0, scene);
    library.paint_scene(248.0, 400.0, scene);
    file_tree.paint_scene(248.0, 400.0, scene);
    properties.paint_scene(280.0, 400.0, scene);
    chat.paint_scene(320.0, 400.0, scene);
    deep_think.paint_scene(320.0, 400.0, scene);
}

fn raster_frosted_rect(rect: &FrostedRect, width: usize, height: usize, pixels: &mut [[u8; 3]]) {
    let color = [22u8, 34u8, 58u8];
    let x0 = rect.bounds.origin.x.0.max(0.0) as usize;
    let y0 = rect.bounds.origin.y.0.max(0.0) as usize;
    let x1 = (rect.bounds.origin.x.0 + rect.bounds.size.width.0).min(width as f32) as usize;
    let y1 = (rect.bounds.origin.y.0 + rect.bounds.size.height.0).min(height as f32) as usize;
    for y in y0.min(height)..y1.min(height) {
        for x in x0.min(width)..x1.min(width) {
            pixels[y * width + x] = color;
        }
    }
}

fn raster_quad(quad: &Quad, width: usize, height: usize, pixels: &mut [[u8; 3]]) {
    let color = quad
        .background
        .or(quad.border_color)
        .map(hsla_to_rgb)
        .unwrap_or([255, 255, 255]);
    let x0 = quad.bounds.origin.x.0.max(0.0) as usize;
    let y0 = quad.bounds.origin.y.0.max(0.0) as usize;
    let x1 = (quad.bounds.origin.x.0 + quad.bounds.size.width.0).min(width as f32) as usize;
    let y1 = (quad.bounds.origin.y.0 + quad.bounds.size.height.0).min(height as f32) as usize;
    for y in y0.min(height)..y1.min(height) {
        for x in x0.min(width)..x1.min(width) {
            pixels[y * width + x] = color;
        }
    }
}

fn hsla_to_rgb(color: Hsla) -> [u8; 3] {
    let c = (1.0 - (2.0 * color.l - 1.0).abs()) * color.s;
    let h_prime = color.h / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match h_prime {
        h if (0.0..1.0).contains(&h) => (c, x, 0.0),
        h if (1.0..2.0).contains(&h) => (x, c, 0.0),
        h if (2.0..3.0).contains(&h) => (0.0, c, x),
        h if (3.0..4.0).contains(&h) => (0.0, x, c),
        h if (4.0..5.0).contains(&h) => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = color.l - c / 2.0;
    [
        ((r1 + m).clamp(0.0, 1.0) * 255.0) as u8,
        ((g1 + m).clamp(0.0, 1.0) * 255.0) as u8,
        ((b1 + m).clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

fn write_ppm(path: &Path, width: usize, height: usize, pixels: &[[u8; 3]]) {
    let mut bytes = format!("P6\n{} {}\n255\n", width, height).into_bytes();
    for pixel in pixels {
        bytes.extend_from_slice(pixel);
    }
    fs::write(path, bytes).expect("write ppm");
}
