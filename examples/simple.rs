fn main() -> anyhow::Result<()> {
    punctum::builder()
        .with_title("Awesome Engine")
        .with_canvas_id("my-viewport")
        .with_size(900, 800)
        .run()
}
