use atrofac_libgui::engine::Engine;
use atrofac_libgui::system::{
    new_system_interface, MenuItem, MenuItemIdx, MenuItemState, StringMenuItem, SystemEvent,
    SystemInterface,
};
use atrofac_library::AfErr;
use std::borrow::Cow;
use std::convert::TryFrom;
use std::time::Duration;

const MENU_ITEM_RELOAD_CONFIG_OFFSET: usize = 1;
const MENU_ITEM_EDIT_CONFIG_OFFSET: usize = 2;
const MENU_ITEM_EDIT_EXIT_OFFSET: usize = 3;
const DEFAULT_INTERVAL_SEC: u32 = 120;

fn apply(engine: &mut Engine, system: &mut impl SystemInterface) -> Result<(), AfErr> {
    engine.apply()?;

    // set the timer
    if let Some(active_plan) = engine.active_plan() {
        let interval_secs = active_plan
            .update_interval_sec
            .unwrap_or(DEFAULT_INTERVAL_SEC);
        system.set_timer(Duration::from_secs(interval_secs as u64))?;
    }
    Ok(())
}

fn load_tray(engine: &Engine, system: &mut impl SystemInterface) -> Result<(), AfErr> {
    system.tray_clear()?;

    let active_plan = engine.active_plan();
    for (_, plan_name) in engine.available_plans() {
        let active = if let Some(active_plan) = active_plan {
            active_plan.name == plan_name
        } else {
            false
        };
        system.tray_add(MenuItem::String(StringMenuItem {
            text: Cow::Borrowed(plan_name.as_str()),
            state: if active {
                MenuItemState::Checked
            } else {
                MenuItemState::Default
            },
        }))?;
    }
    system.tray_add(MenuItem::Separator)?;
    system.tray_add(MenuItem::String(StringMenuItem {
        text: "Reload configuration".into(),
        state: MenuItemState::Default,
    }))?;
    system.tray_add(MenuItem::String(StringMenuItem {
        text: "Edit configuration".into(),
        state: MenuItemState::Default,
    }))?;
    system.tray_add(MenuItem::String(StringMenuItem {
        text: "Quit application".into(),
        state: MenuItemState::Default,
    }))?;
    Ok(())
}

fn on_tray(
    menu_item_id: MenuItemIdx,
    engine: &mut Engine,
    system: &mut impl SystemInterface,
) -> Result<(), AfErr> {
    let index_usize = usize::try_from(menu_item_id.id())?;
    let number_of_plans = engine.number_of_plans();
    if index_usize >= number_of_plans {
        // not a plan
        let offset = index_usize - number_of_plans;
        match offset {
            MENU_ITEM_RELOAD_CONFIG_OFFSET => {
                engine.load_configuration()?;
                load_tray(engine, system)?;
                apply(engine, system)?;
                Ok(())
            }
            MENU_ITEM_EDIT_CONFIG_OFFSET => {
                let config_file = engine.config_file();
                system.edit(config_file)?;
                Ok(())
            }
            MENU_ITEM_EDIT_EXIT_OFFSET => {
                system.quit()?;
                Ok(())
            }
            _ => Err(AfErr::from(format!("Unknown menu item offset {}.", offset))),
        }
    } else {
        // it's a plan
        if let Some(plan_name) = engine.plan_by_index(menu_item_id.id() as usize).cloned() {
            engine.set_active_plan(plan_name);
            // when the plan has been changed, save the configuration
            engine.save_configuration()?;
            apply(engine, system)?;
            // reload tray
            load_tray(engine, system)?;
            Ok(())
        } else {
            Err(AfErr::from(format!(
                "Plan #{} not found.",
                menu_item_id.id()
            )))
        }
    }
}

fn run_main_with_error(
    engine: &mut Engine,
    system: &mut impl SystemInterface,
) -> Result<(), AfErr> {
    engine.load_configuration()?;
    apply(engine, system)?;
    load_tray(engine, system)?;
    system.tray_tooltip("Control fan curve and power profile for Asus Zephyrus ROG G14.")?;

    // loop
    loop {
        let event = system.receive_event()?;
        if let Some(event) = event {
            match event {
                SystemEvent::OnTimer => {
                    apply(engine, system)?;
                }
                SystemEvent::OnTray(menu_item_id) => {
                    on_tray(menu_item_id, engine, system)?;
                }
            }
        } else {
            // finish
            return Ok(());
        }
    }
}

fn run_main(engine: &mut Engine, system: &mut impl SystemInterface) {
    if let Err(err) = run_main_with_error(engine, system) {
        system
            .show_err_message("Error", &format!("{}", err))
            .expect("Unable to display error message");
    }
}

fn main() {
    let mut system = new_system_interface().expect("Unable to create system interface");
    let mut engine = Engine::new().expect("Unable to create engine.");
    run_main(&mut engine, &mut system);
}