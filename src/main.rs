use component::Interface;
use component::sub::com::Component;

slint::include_modules!();

fn main() -> anyhow::Result<()> {
    let component = Component;
    let ui = AppWindow::new()?;

    ui.on_request_increase_value({
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_counter(ui.get_counter() + 1);
        }
    });

    env_logger::init();
    component.interface()?;
    ui.run()?;

    Ok(())
}
