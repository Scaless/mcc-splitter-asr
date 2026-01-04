extern crate alloc;

use alloc::{format, string::ToString};
use asr::{
    deep_pointer::DeepPointer, print_message, settings::Gui, string::ArrayCString, watcher::Pair,
    watcher::Watcher, Process,
};

type DeepPtr = DeepPointer<8>; // Helper type for deep pointer which can store a chain of up to 8 ptrs

asr::async_main!(stable);
//asr::panic_handler!();

#[derive(Gui)]
struct Settings {
    /// Individual Level Mode
    #[default = false]
    individual_level_mode: Pair<bool>,
}

struct GameState {
    level_name: Watcher<ArrayCString<32>>,
    current_tick: Watcher<u32>,
    igt: Watcher<u32>,
}

impl GameState {
    fn new() -> Self {
        Self {
            level_name: Watcher::new(),
            current_tick: Watcher::new(),
            igt: Watcher::new(),
        }
    }
}

async fn main() {
    let mut settings = Settings::register();
    let mut state = GameState::new();

    loop {
        let (process, exe_name) = asr::future::retry(|| {
            [
                "MCC-Win64-Shipping.exe",
                "MCC-Win64-Shipping-WinStore.exe",
                "MCCWinStore-Win64-Shipping.exe",
            ]
            .into_iter()
            .find_map(|name| Process::attach(name).map(|p| (p, name)))
        })
        .await;

        print_message(&format!("Attached to: {}", exe_name));

        let mcc_module = process.get_module_address(exe_name).unwrap();

        let mcc_version = asr::file_format::pe::FileVersion::read(&process, mcc_module).unwrap();

        let mcc_version_str = &format!(
            "{}.{}.{}.{}",
            mcc_version.major_version,
            mcc_version.minor_version,
            mcc_version.build_part,
            mcc_version.private_part
        );

        print_message(mcc_version_str);

        process
            .until_closes(async {
                let mut first_time: bool = true;

                // The main loop!
                loop {
                    let h1dll = process.get_module_address("halo1.dll").unwrap_or_default();

                    // print_message(&format!("halo1.dll is loaded at: {}", h1dll));

                    let h1_map_offset: u64 = 0x2B22744;

                    let levelname_dp: DeepPtr =
                        DeepPointer::new_64bit(h1dll, &[h1_map_offset + 0x20]);
                    let curtick_dp: DeepPtr = DeepPointer::new_64bit(h1dll, &[0x2B6F5E4]);
                    let igt_dp: DeepPtr = DeepPointer::new_64bit(h1dll, &[0x2EA31D4]);

                    // Updates
                    settings.update();

                    let level_name: ArrayCString<32> = levelname_dp.deref(&process).unwrap();
                    let curtick: u32 = curtick_dp.deref(&process).unwrap();
                    let igt: u32 = igt_dp.deref(&process).unwrap();

                    let level_pair = state.level_name.update(Some(level_name));
                    let igt_pair = state.igt.update(Some(igt));

                    state.current_tick.update(Some(curtick));

                    if let Some(pair) = level_pair {
                        //let old_utf8 = pair.old.validate_utf8().unwrap();
                        let new_utf8 = pair.current.validate_utf8().unwrap();

                        if first_time || pair.changed() {
                            if asr::timer::state() == asr::timer::TimerState::Running {
                                //asr::print_message(&format!("Level changed from {:?} to {:?}, splitting", old_utf8, new_utf8));
                                //asr::timer::resume_game_time();
                                //asr::timer::split();
                            }
                        }

                        asr::timer::set_variable("MCC Version", mcc_version_str);
                        asr::timer::set_variable("Level Name", new_utf8);
                        asr::timer::set_variable("Current Tick", &curtick.to_string());
                        asr::timer::set_variable("IGT", &igt.to_string());

                        if !level_name.is_empty() {
                            if let Some(igt_p) = igt_pair {
                                if igt_p.current < igt_p.old {
                                    asr::print_message("Level Was Restarted: Resetting Timer");
                                    asr::timer::reset();
                                    asr::timer::start();
                                }
                            }
                            if igt <= 1 && asr::timer::state() != asr::timer::TimerState::Running {
                                asr::print_message("Level Loaded: Resetting Timer");
                                asr::timer::start();
                            }
                        } else {
                            if asr::timer::state() == asr::timer::TimerState::Running {
                                asr::print_message("No Level Loaded: Resetting Timer");
                                asr::timer::reset()
                            }
                        }
                    }

                    first_time = false;

                    // Go around again
                    asr::future::next_tick().await;
                }
            })
            .await;
    }
}

