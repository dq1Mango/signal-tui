use ratatui::Frame;
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};
use std::{thread, time};

struct App {
    // We need to hold the render state.
    image: StatefulProtocol,
}

struct Settings {
    borders: bool,
}

struct Test {
    settings: Settings,
}

fn update_settings(settings: Settings) {
    settings.borders = true;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut settigns = Settings { borders: true };

    let test = Test { settings: settigns };

    settigns.borders = false;
    update_settings(settings);
    settigns.borders = false;
    println!("{}", settigns.borders);
    // let backend = TestBackend::new(80, 30);
    let mut terminal = ratatui::init();

    // Should use Picker::from_query_stdio() to get the font size and protocol,
    // but we can't put that here because that would break doctests!
    let picker = Picker::from_query_stdio()?;

    // Load an image with the image crate.
    let dyn_img = image::ImageReader::open("./assets/city-window.png")?.decode()?;

    // Create the Protocol which will be used by the widget.
    let image = picker.new_resize_protocol(dyn_img);

    let mut app = App { image };

    // This would be your typical `loop {` in a real app:
    terminal.draw(|f| ui(f, &mut app))?;

    let ten_millis = time::Duration::from_secs(1);

    thread::sleep(ten_millis);
    // It is recommended to handle the encoding result
    app.image.last_encoding_result().unwrap()?;

    ratatui::restore();
    Ok(())
}

fn ui(f: &mut Frame<'_>, app: &mut App) {
    // The image widget.
    let image = StatefulImage::default();
    // Render with the protocol state.
    f.render_stateful_widget(image, f.area(), &mut app.image);
}
