mod splitter;

extern crate alloc;

use alloc::{format, string::ToString};
use asr::file_format::pe::FileVersion;
use asr::timer::TimerState;
use asr::{deep_pointer::DeepPointer, print_message, settings::Gui, string::ArrayCString, watcher::Watcher, Process};
use splitter::{H1Checklist, MCCGame, SplitterState, *};

// Helper type for deep pointer which can store a chain of up to 8 ptrs
type DeepPtr = DeepPointer<8>;

asr::async_main!(stable);
//asr::panic_handler!();

macro_rules! current {
    ($watcher:expr) => {
        $watcher.pair.as_ref().map(|p| p.current)
    };
}

macro_rules! old {
    ($watcher:expr) => {
        $watcher.pair.as_ref().map(|p| p.old)
    };
}

macro_rules! changed {
    ($watcher:expr) => {
        $watcher.pair.as_ref().map(|p| p.current != p.old).unwrap_or(false)
    };
}

macro_rules! changed_to {
    ($watcher:expr, $val:expr) => {
        $watcher.pair.as_ref().map(|p| p.current == $val && p.old != $val).unwrap_or(false)
    };
}

macro_rules! changed_from {
    ($watcher:expr, $val:expr) => {
        $watcher.pair.as_ref().map(|p| p.old == $val && p.current != $val).unwrap_or(false)
    };
}

macro_rules! changed_from_to {
    ($watcher:expr, $from:expr, $to:expr) => {
        $watcher.pair.as_ref().map(|p| p.old == $from && p.current == $to).unwrap_or(false)
    };
}

#[derive(Gui)]
struct Settings {
    #[default = false]
    /// Individual Level mode
    il_mode: bool,

    #[default = false]
    /// Level Loop mode (for TBx10)
    loop_mode: bool,

    #[default = false]
    /// Split on unique "Loading... Done"'s
    bsp_mode: bool,

    #[default = false]
    /// Split on non-unique loads too
    bsp_cache: bool,

    #[default = false]
    /// Use in-game competitive timer splits
    comp_splits: bool,

    #[default = false]
    /// Don't pause timer on pause screen (H3 Coop)
    h3_coop: bool,

    #[default = false]
    /// Start full-game runs on any level
    any_level: bool,

    #[default = true]
    /// Pause when in Main Menu
    menu_pause: bool,

    #[default = false]
    /// Split when loading a level from main menu
    sq_split: bool,

    #[default = false]
    /// Start the timer on custom maps (Halo: CE Only)
    any_start: bool,

    #[default = false]
    /// Enable Death Counter
    death_counter: bool,

    #[default = false]
    /// Add exact IGT on mission restart
    igt_add: bool,

    #[default = false]
    /// IGT Debug - Forces IGT sync regardless of game
    igt_mode: bool,
}

#[derive(Default)]
struct GameDLLs {
    exe_mcc: asr::Address,
    dll_halo1: asr::Address,
    dll_halo2: asr::Address,
    dll_halo3: asr::Address,
    dll_halo4: asr::Address,
    dll_halo3_odst: asr::Address,
    dll_halo_reach: asr::Address,
}

#[derive(Default)]
struct GamePointers {
    // State - MCC
    mcc_loadindicator: DeepPtr,
    mcc_menuindicator: DeepPtr,
    mcc_pauseindicator: DeepPtr,
    mcc_pgcrindicator: DeepPtr,
    mcc_gameindicator: DeepPtr,
    mcc_igt_float: DeepPtr,
    mcc_comptimerstate: DeepPtr,

    // State - Halo 1
    h1_tickcounter: DeepPtr,
    h1_igt: DeepPtr,
    h1_bspstate: DeepPtr,
    h1_levelname: DeepPtr,
    h1_gamewon: DeepPtr,
    h1_cinematic: DeepPtr,
    h1_cutsceneskip: DeepPtr,
    h1_xpos: DeepPtr,
    h1_ypos: DeepPtr,
    h1_fadetick: DeepPtr,
    h1_fadelength: DeepPtr,
    h1_fadebyte: DeepPtr,
    h1_deathflag: DeepPtr,
    h1_checksum: DeepPtr,
    h1_aflags: DeepPtr,

    // State - Halo 2
    h2_levelname: DeepPtr,
    h2_igt: DeepPtr,
    h2_bspstate: DeepPtr,
    h2_deathflag: DeepPtr,
    h2_tickcounter: DeepPtr,
    h2_graphics: DeepPtr,
    h2_fadebyte: DeepPtr,
    h2_letterbox: DeepPtr,
    h2_xpos: DeepPtr,
    h2_ypos: DeepPtr,
    h2_fadetick: DeepPtr,
    h2_fadelength: DeepPtr,

    // State - Halo 3
    h3_levelname: DeepPtr,
    h3_theatertime: DeepPtr,
    h3_tickcounter: DeepPtr,
    h3_bspstate: DeepPtr,
    h3_deathflag: DeepPtr,

    // State - Halo Reach
    hr_levelname: DeepPtr,
    hr_bspstate: DeepPtr,
    hr_deathflag: DeepPtr,

    // State - ODST
    odst_levelname: DeepPtr,
    odst_streets: DeepPtr,
    odst_bspstate: DeepPtr,
    odst_deathflag: DeepPtr,

    // State - Halo 4
    h4_levelname: DeepPtr,
    h4_bspstate: DeepPtr,

    // Version-dependent constants
    h1_checklist: H1Checklist,
    fadescale: f64,
}

#[derive(Default)]
struct GameState {
    // MCC
    mcc_loadindicator: Watcher<u8>,
    mcc_menuindicator: Watcher<u8>,
    mcc_pauseindicator: Watcher<u8>,
    mcc_pgcrindicator: Watcher<u8>,
    mcc_gameindicator: Watcher<MCCGame>,
    mcc_igt_float: Watcher<f32>,
    mcc_comptimerstate: Watcher<u32>,

    // Halo 1
    h1_tickcounter: Watcher<u32>,
    h1_igt: Watcher<u32>,
    h1_bspstate: Watcher<u8>,
    h1_levelname: Watcher<ArrayCString<32>>,
    h1_gamewon: Watcher<bool>,
    h1_cinematic: Watcher<bool>,
    h1_cutsceneskip: Watcher<bool>,
    h1_xpos: Watcher<f32>,
    h1_ypos: Watcher<f32>,
    h1_fadetick: Watcher<u32>,
    h1_fadelength: Watcher<u16>,
    h1_fadebyte: Watcher<u8>,
    h1_deathflag: Watcher<bool>,
    h1_checksum: Watcher<u32>,
    h1_aflags: Watcher<u8>,

    // Halo 2
    h2_levelname: Watcher<ArrayCString<3>>,
    h2_igt: Watcher<u32>,
    h2_bspstate: Watcher<u8>,
    h2_deathflag: Watcher<bool>,
    h2_tickcounter: Watcher<u32>,
    h2_graphics: Watcher<u8>,
    h2_fadebyte: Watcher<u8>,
    h2_letterbox: Watcher<f32>,
    h2_xpos: Watcher<f32>,
    h2_ypos: Watcher<f32>,
    h2_fadetick: Watcher<u32>,
    h2_fadelength: Watcher<u16>,

    // Halo 3
    h3_levelname: Watcher<ArrayCString<3>>,
    h3_theatertime: Watcher<u32>,
    h3_tickcounter: Watcher<u32>,
    h3_bspstate: Watcher<u64>,
    h3_deathflag: Watcher<bool>,

    // Halo Reach
    hr_levelname: Watcher<ArrayCString<3>>,
    hr_bspstate: Watcher<u32>,
    hr_deathflag: Watcher<bool>,

    // ODST
    odst_levelname: Watcher<ArrayCString<4>>,
    odst_streets: Watcher<u8>,
    odst_bspstate: Watcher<u32>,
    odst_deathflag: Watcher<bool>,

    // Halo 4
    h4_levelname: Watcher<ArrayCString<3>>,
    h4_bspstate: Watcher<u64>,
}

fn update_game_pointers(is_winstore: bool, mcc_version: FileVersion, dlls: &GameDLLs, ptrs: &mut GamePointers) {
    if is_winstore && mcc_version.minor_version < 3272 {
        panic!("Invalid WinStore version should have been handled!");
    }

    *ptrs = GamePointers::default();

    match mcc_version.minor_version {
        2448 => {
            ptrs.h1_checklist = H1Checklist {
                a10: 2495112808,
                a30: 1196246201,
                a50: 3037603536,
                b30: 682311759,
                b40: 326064131,
                c10: 645721511,
                c20: 540616268,
                c40: 1500399674,
                d20: 2770760039,
                d40: 1695151528,
            };
            ptrs.fadescale = 0.183;
        }
        2645 => {
            ptrs.h1_checklist = H1Checklist {
                a10: 4031641132,
                a30: 1497905037,
                a50: 2613596386,
                b30: 4057206713,
                b40: 2439716616,
                c10: 2597150717,
                c20: 1656675814,
                c40: 1573304389,
                d20: 1507739304,
                d40: 2038583061,
            };
            ptrs.fadescale = 0.067;
        }
        2904 => {
            ptrs.h1_checklist = H1Checklist {
                a10: 89028072,
                a30: 1083179843,
                a50: 2623582826,
                b30: 1895318681,
                b40: 1935970024,
                c10: 974037405,
                c20: 714510620,
                c40: 2859044941,
                d20: 1178559651,
                d40: 3253884125,
            };
            ptrs.fadescale = 0.067;
        }
        2969 => {
            ptrs.h1_checklist = H1Checklist {
                a10: 2023477633,
                a30: 1197744442,
                a50: 522123179,
                b30: 2022995318,
                b40: 4112928798,
                c10: 4250424451,
                c20: 1165450382,
                c40: 2733116763,
                d20: 1722772470,
                d40: 3775314541,
            };
            ptrs.fadescale = 0.067;
        }
        3073 => {
            ptrs.h1_checklist = H1Checklist {
                a10: 3589325267,
                a30: 3649693672,
                a50: 1186687708,
                b30: 1551598635,
                b40: 1100623455,
                c10: 3494823778,
                c20: 2445460720,
                c40: 3759075146,
                d20: 3442848200,
                d40: 1751474532,
            };
            ptrs.fadescale = 0.067;
        }
        3272 | 3385 | 3495 | 3498 | 3528 | _ /* Unknown Version, attempt to use latest */ => {
            ptrs.h1_checklist = H1Checklist {
                a10: 1731967100,
                a30: 2334900663,
                a50: 2345488806,
                b30: 389775619,
                b40: 232036917,
                c10: 3544120777,
                c20: 2188406812,
                c40: 687169669,
                d20: 485256620,
                d40: 1783204841,
            };
            ptrs.fadescale = 0.067;
        }
    }

    match mcc_version.minor_version {
        2448 => {
            // MCC - Steam only
            let menustate: u64 = 0x3A24FC4;
            ptrs.mcc_loadindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate]);
            ptrs.mcc_menuindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0x11]);
            ptrs.mcc_pauseindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0xA]);
            ptrs.mcc_pgcrindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0xB]);
            ptrs.mcc_gameindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3A253A0, 0x0]);
            ptrs.mcc_igt_float = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3A25188]);
            ptrs.mcc_comptimerstate = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3A254B0, 0x1A4]);

            // Halo 1
            const H1_GLOBALS: u64 = 0x2AF10D0;
            const H1_MAP: u64 = 0x2A4BC04;
            const H1_CINFLAGS: u64 = 0x2AF1868;
            const H1_COORDS: u64 = 0x2A57E74;
            const H1_FADE: u64 = 0x2B81CE8;

            ptrs.h1_tickcounter = DeepPtr::new_64bit(dlls.dll_halo1, &[0x2B58A24]);
            ptrs.h1_igt = DeepPtr::new_64bit(dlls.dll_halo1, &[0x2AF477C]);
            ptrs.h1_bspstate = DeepPtr::new_64bit(dlls.dll_halo1, &[0x19F0400]);
            ptrs.h1_levelname = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x20]);
            ptrs.h1_gamewon = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_GLOBALS + 0x1]);
            ptrs.h1_cinematic = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_CINFLAGS, 0x0A]);
            ptrs.h1_cutsceneskip = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_CINFLAGS, 0x0B]);
            ptrs.h1_xpos = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_COORDS]);
            ptrs.h1_ypos = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_COORDS + 0x4]);
            ptrs.h1_fadetick = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C0]);
            ptrs.h1_fadelength = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C4]);
            ptrs.h1_fadebyte = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C6]);
            ptrs.h1_deathflag = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_GLOBALS + 0x17]);
            ptrs.h1_checksum = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x64]);
            ptrs.h1_aflags = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x68]);

            // Halo 2
            const H2_CINFLAGS: u64 = 0x143ACA0;
            const H2_COORDS: u64 = 0xDA5CD8;
            const H2_FADE: u64 = 0x13DFC58;

            ptrs.h2_levelname = DeepPtr::new_64bit(dlls.dll_halo2, &[0xE63FB3]);
            ptrs.h2_igt = DeepPtr::new_64bit(dlls.dll_halo2, &[0xE22F40]);
            ptrs.h2_bspstate = DeepPtr::new_64bit(dlls.dll_halo2, &[0xCD7D74]);
            ptrs.h2_deathflag = DeepPtr::new_64bit(dlls.dll_halo2, &[0xDA6140, -0xEFi64 as u64]);
            ptrs.h2_tickcounter = DeepPtr::new_64bit(dlls.dll_halo2, &[0xE63144]);
            ptrs.h2_graphics = DeepPtr::new_64bit(dlls.dll_halo2, &[0xCFB918]);
            ptrs.h2_fadebyte = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_CINFLAGS, -0x92Ei64 as u64]);
            ptrs.h2_letterbox = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_CINFLAGS, -0x938i64 as u64]);
            ptrs.h2_xpos = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_COORDS]);
            ptrs.h2_ypos = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_COORDS + 0x4]);
            ptrs.h2_fadetick = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_FADE, 0x0]);
            ptrs.h2_fadelength = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_FADE, 0x4]);

            // Halo 3
            ptrs.h3_levelname = DeepPtr::new_64bit(dlls.dll_halo3, &[0x1D2C460]);
            ptrs.h3_theatertime = DeepPtr::new_64bit(dlls.dll_halo3, &[0x1DDC3BC]);
            ptrs.h3_tickcounter = DeepPtr::new_64bit(dlls.dll_halo3, &[0x2961E0C]);
            ptrs.h3_bspstate = DeepPtr::new_64bit(dlls.dll_halo3, &[0x9F3EF0, 0x2C]);
            ptrs.h3_deathflag = DeepPtr::new_64bit(dlls.dll_halo3, &[0x1CB15C8, 0x1051D]);

            // Halo Reach
            ptrs.hr_levelname = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x2868777]);
            ptrs.hr_bspstate = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x36778E0]);
            ptrs.hr_deathflag = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0xEEFEB0, 0x544249]);

            // ODST
            ptrs.odst_levelname = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x1CDF200]);
            ptrs.odst_streets = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x1DB2568]);
            ptrs.odst_bspstate = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x2E46964]);
            ptrs.odst_deathflag = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0xE8520C, -0x913i64 as u64]);

            // Halo 4
            ptrs.h4_levelname = DeepPtr::new_64bit(dlls.dll_halo4, &[0x276ACA3]);
            ptrs.h4_bspstate = DeepPtr::new_64bit(dlls.dll_halo4, &[0x2441AB8, -0x560i64 as u64]);
        }

        2645 => {
            // MCC - Steam only
            let menustate: u64 = 0x3B80E64;
            ptrs.mcc_loadindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate]);
            ptrs.mcc_menuindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0x11]);
            ptrs.mcc_pauseindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0xB]);
            ptrs.mcc_pgcrindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0xC]);
            ptrs.mcc_gameindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3B81270, 0x0]);
            ptrs.mcc_igt_float = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3B80FF8]);
            ptrs.mcc_comptimerstate = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3B81380, 0x1A4]);

            // Halo 1
            const H1_GLOBALS: u64 = 0x2AF8240;
            const H1_MAP: u64 = 0x2A52D84;
            const H1_CINFLAGS: u64 = 0x2AF89B8;
            const H1_COORDS: u64 = 0x2A5EFF4;
            const H1_FADE: u64 = 0x2B88E58;

            ptrs.h1_tickcounter = DeepPtr::new_64bit(dlls.dll_halo1, &[0x2B5FC04]);
            ptrs.h1_igt = DeepPtr::new_64bit(dlls.dll_halo1, &[0x2AFB954]);
            ptrs.h1_bspstate = DeepPtr::new_64bit(dlls.dll_halo1, &[0x19F748C]);
            ptrs.h1_levelname = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x20]);
            ptrs.h1_gamewon = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_GLOBALS + 0x1]);
            ptrs.h1_cinematic = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_CINFLAGS, 0x0A]);
            ptrs.h1_cutsceneskip = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_CINFLAGS, 0x0B]);
            ptrs.h1_xpos = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_COORDS]);
            ptrs.h1_ypos = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_COORDS + 0x4]);
            ptrs.h1_fadetick = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C0]);
            ptrs.h1_fadelength = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C4]);
            ptrs.h1_fadebyte = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C6]);
            ptrs.h1_deathflag = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_GLOBALS + 0x17]);
            ptrs.h1_checksum = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x64]);
            ptrs.h1_aflags = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x68]);

            // Halo 2
            const H2_CINFLAGS: u64 = 0x15186A0;
            const H2_COORDS: u64 = 0xD523A8;
            const H2_FADE: u64 = 0x14BD450;

            ptrs.h2_levelname = DeepPtr::new_64bit(dlls.dll_halo2, &[0xD42E68]);
            ptrs.h2_igt = DeepPtr::new_64bit(dlls.dll_halo2, &[0x1475C10]);
            ptrs.h2_bspstate = DeepPtr::new_64bit(dlls.dll_halo2, &[0xCA4D74]);
            ptrs.h2_deathflag = DeepPtr::new_64bit(dlls.dll_halo2, &[0xD52800, -0xEFi64 as u64]);
            ptrs.h2_tickcounter = DeepPtr::new_64bit(dlls.dll_halo2, &[0x14B5DE4]);
            ptrs.h2_graphics = DeepPtr::new_64bit(dlls.dll_halo2, &[0xCC74A8]);
            ptrs.h2_fadebyte = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_CINFLAGS, -0x92Ei64 as u64]);
            ptrs.h2_letterbox = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_CINFLAGS, -0x938i64 as u64]);
            ptrs.h2_xpos = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_COORDS]);
            ptrs.h2_ypos = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_COORDS + 0x4]);
            ptrs.h2_fadetick = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_FADE, 0x0]);
            ptrs.h2_fadelength = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_FADE, 0x4]);

            // Halo 3
            ptrs.h3_levelname = DeepPtr::new_64bit(dlls.dll_halo3, &[0x1E0D358]);
            ptrs.h3_theatertime = DeepPtr::new_64bit(dlls.dll_halo3, &[0x1EDAA9C]);
            ptrs.h3_tickcounter = DeepPtr::new_64bit(dlls.dll_halo3, &[0x2A1F34C]);
            ptrs.h3_bspstate = DeepPtr::new_64bit(dlls.dll_halo3, &[0x9A4BA0, 0x2C]);
            ptrs.h3_deathflag = DeepPtr::new_64bit(dlls.dll_halo3, &[0x1D91E68, 0x1077D]);

            // Halo Reach
            ptrs.hr_levelname = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x2907107]);
            ptrs.hr_bspstate = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x3716270]);
            ptrs.hr_deathflag = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0xEEF330, 0x594249]);

            // ODST
            ptrs.odst_levelname = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x2020CA8]);
            ptrs.odst_streets = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x2116FD8]);
            ptrs.odst_bspstate = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x2F91A9C]);
            ptrs.odst_deathflag = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0xF3020C, -0x913i64 as u64]);

            // Halo 4
            ptrs.h4_levelname = DeepPtr::new_64bit(dlls.dll_halo4, &[0x2836433]);
            ptrs.h4_bspstate = DeepPtr::new_64bit(dlls.dll_halo4, &[0x2472A88]);
        }

        2904 => {
            // MCC - Steam only
            let menustate: u64 = 0x3F7BAAD;
            ptrs.mcc_loadindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate]);
            ptrs.mcc_menuindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0x8]);
            ptrs.mcc_pauseindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0x5]);
            ptrs.mcc_pgcrindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0x6]);
            ptrs.mcc_gameindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3F7C380, 0x0]);
            ptrs.mcc_igt_float = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3F7C33C]);
            ptrs.mcc_comptimerstate = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3F7C358, 0x1A4]);

            // Halo 1
            const H1_GLOBALS: u64 = 0x2B611A0;
            const H1_MAP: u64 = 0x2D66A24;
            const H1_CINFLAGS: u64 = 0x2E773D8;
            const H1_COORDS: u64 = 0x2D7313C;
            const H1_FADE: u64 = 0x2E7F868;

            ptrs.h1_tickcounter = DeepPtr::new_64bit(dlls.dll_halo1, &[0x2B88764]);
            ptrs.h1_igt = DeepPtr::new_64bit(dlls.dll_halo1, &[0x2E7A354]);
            ptrs.h1_bspstate = DeepPtr::new_64bit(dlls.dll_halo1, &[0x1B661CC]);
            ptrs.h1_levelname = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x20]);
            ptrs.h1_gamewon = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_GLOBALS + 0x1]);
            ptrs.h1_cinematic = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_CINFLAGS, 0x0A]);
            ptrs.h1_cutsceneskip = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_CINFLAGS, 0x0B]);
            ptrs.h1_xpos = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_COORDS]);
            ptrs.h1_ypos = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_COORDS + 0x4]);
            ptrs.h1_fadetick = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C0]);
            ptrs.h1_fadelength = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C4]);
            ptrs.h1_fadebyte = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C6]);
            ptrs.h1_deathflag = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_GLOBALS + 0x17]);
            ptrs.h1_checksum = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x64]);
            ptrs.h1_aflags = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x68]);

            // Halo 2
            const H2_CINFLAGS: u64 = 0x1520498;
            const H2_COORDS: u64 = 0xD5A148;
            const H2_FADE: u64 = 0x14C5228;

            ptrs.h2_levelname = DeepPtr::new_64bit(dlls.dll_halo2, &[0xD4ABF8]);
            ptrs.h2_igt = DeepPtr::new_64bit(dlls.dll_halo2, &[0x147D9F0]);
            ptrs.h2_bspstate = DeepPtr::new_64bit(dlls.dll_halo2, &[0xCACD74]);
            ptrs.h2_deathflag = DeepPtr::new_64bit(dlls.dll_halo2, &[0xD5A5A0, -0xEFi64 as u64]);
            ptrs.h2_tickcounter = DeepPtr::new_64bit(dlls.dll_halo2, &[0x14BDBC4]);
            ptrs.h2_graphics = DeepPtr::new_64bit(dlls.dll_halo2, &[0xCCF280]);
            ptrs.h2_fadebyte = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_CINFLAGS, -0x92Ei64 as u64]);
            ptrs.h2_letterbox = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_CINFLAGS, -0x938i64 as u64]);
            ptrs.h2_xpos = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_COORDS]);
            ptrs.h2_ypos = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_COORDS + 0x4]);
            ptrs.h2_fadetick = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_FADE, 0x0]);
            ptrs.h2_fadelength = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_FADE, 0x4]);

            // Halo 3
            ptrs.h3_levelname = DeepPtr::new_64bit(dlls.dll_halo3, &[0x1E092E8]);
            ptrs.h3_theatertime = DeepPtr::new_64bit(dlls.dll_halo3, &[0x1E9B4BC]);
            ptrs.h3_tickcounter = DeepPtr::new_64bit(dlls.dll_halo3, &[0x29E194C]);
            ptrs.h3_bspstate = DeepPtr::new_64bit(dlls.dll_halo3, &[0x99FCA0, 0x2C]);
            ptrs.h3_deathflag = DeepPtr::new_64bit(dlls.dll_halo3, &[0x1D8DF48, 0x1073D]);

            // Halo Reach
            ptrs.hr_levelname = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x28A4C3F]);
            ptrs.hr_bspstate = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x3719E24]);
            ptrs.hr_deathflag = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x23CC7D8, 0x1F419]);

            // ODST
            ptrs.odst_levelname = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x202EA58]);
            ptrs.odst_streets = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x21353D8]);
            ptrs.odst_bspstate = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x2F9FD4C]);
            ptrs.odst_deathflag = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0xF3EB8C, -0x913i64 as u64]);

            // Halo 4
            ptrs.h4_levelname = DeepPtr::new_64bit(dlls.dll_halo4, &[0x29A3743]);
            ptrs.h4_bspstate = DeepPtr::new_64bit(dlls.dll_halo4, &[0x25DC188]);
        }

        2969 => {
            // MCC - Steam only
            let menustate: u64 = 0x3F9446C;
            ptrs.mcc_loadindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate]);
            ptrs.mcc_menuindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0x11]);
            ptrs.mcc_pauseindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0xB]);
            ptrs.mcc_pgcrindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0xC]);
            ptrs.mcc_gameindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3F94E90, 0x0]);
            ptrs.mcc_igt_float = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3F94F88]);
            ptrs.mcc_comptimerstate = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3F94F60, 0x1A4]);

            // Halo 1
            const H1_GLOBALS: u64 = 0x2CC5860;
            const H1_MAP: u64 = 0x2EEB024;
            const H1_CINFLAGS: u64 = 0x2FFBD28;
            const H1_COORDS: u64 = 0x1DF5FF8;
            const H1_FADE: u64 = 0x30041A8;

            ptrs.h1_tickcounter = DeepPtr::new_64bit(dlls.dll_halo1, &[0x2CECFD4]);
            ptrs.h1_igt = DeepPtr::new_64bit(dlls.dll_halo1, &[0x14872C0]);
            ptrs.h1_bspstate = DeepPtr::new_64bit(dlls.dll_halo1, &[0x1CE4920]);
            ptrs.h1_levelname = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x20]);
            ptrs.h1_gamewon = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_GLOBALS + 0x1]);
            ptrs.h1_cinematic = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_CINFLAGS, 0x0A]);
            ptrs.h1_cutsceneskip = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_CINFLAGS, 0x0B]);
            ptrs.h1_xpos = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_COORDS]);
            ptrs.h1_ypos = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_COORDS + 0x4]);
            ptrs.h1_fadetick = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C0]);
            ptrs.h1_fadelength = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C4]);
            ptrs.h1_fadebyte = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C6]);
            ptrs.h1_deathflag = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_GLOBALS + 0x17]);
            ptrs.h1_checksum = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x64]);
            ptrs.h1_aflags = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x68]);

            // Halo 2
            const H2_CINFLAGS: u64 = 0x14D9448;
            const H2_COORDS: u64 = 0xD63A28;
            const H2_FADE: u64 = 0x14CEB68;

            ptrs.h2_levelname = DeepPtr::new_64bit(dlls.dll_halo2, &[0xD54498]);
            ptrs.h2_igt = DeepPtr::new_64bit(dlls.dll_halo2, &[0x14872C0]);
            ptrs.h2_bspstate = DeepPtr::new_64bit(dlls.dll_halo2, &[0xCB2D74]);
            ptrs.h2_deathflag = DeepPtr::new_64bit(dlls.dll_halo2, &[0xD63E80, -0xEFi64 as u64]);
            ptrs.h2_tickcounter = DeepPtr::new_64bit(dlls.dll_halo2, &[0x14C7494]);
            ptrs.h2_graphics = DeepPtr::new_64bit(dlls.dll_halo2, &[0xCD8998]);
            ptrs.h2_fadebyte = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_CINFLAGS, -0x92Ei64 as u64]);
            ptrs.h2_letterbox = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_CINFLAGS, -0x938i64 as u64]);
            ptrs.h2_xpos = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_COORDS]);
            ptrs.h2_ypos = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_COORDS + 0x4]);
            ptrs.h2_fadetick = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_FADE, 0x0]);
            ptrs.h2_fadelength = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_FADE, 0x4]);

            // Halo 3
            ptrs.h3_levelname = DeepPtr::new_64bit(dlls.dll_halo3, &[0x1EABB78]);
            ptrs.h3_theatertime = DeepPtr::new_64bit(dlls.dll_halo3, &[0x1F3DD5C]);
            ptrs.h3_tickcounter = DeepPtr::new_64bit(dlls.dll_halo3, &[0x2B4178C]);
            ptrs.h3_bspstate = DeepPtr::new_64bit(dlls.dll_halo3, &[0xA41D20, 0x2C]);
            ptrs.h3_deathflag = DeepPtr::new_64bit(dlls.dll_halo3, &[0x1E30758, 0x1074D]);

            // Halo Reach
            ptrs.hr_levelname = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x2A39A8F]);
            ptrs.hr_bspstate = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x3BB32A0]);
            ptrs.hr_deathflag = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x2514A88, 0x1F419]);

            // ODST
            ptrs.odst_levelname = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x20D68F8]);
            ptrs.odst_streets = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x21DD308]);
            ptrs.odst_bspstate = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x3417D4C]);
            ptrs.odst_deathflag = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0xFB940C, -0x913i64 as u64]);

            // Halo 4
            ptrs.h4_levelname = DeepPtr::new_64bit(dlls.dll_halo4, &[0x2B03887]);
            ptrs.h4_bspstate = DeepPtr::new_64bit(dlls.dll_halo4, &[0x27564B0]);
        }

        3073 => {
            // MCC - Steam only (Legacy Steam Support - No Winstore)
            let menustate: u64 = 0x401B76C;
            ptrs.mcc_loadindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate]);
            ptrs.mcc_menuindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0x11]);
            ptrs.mcc_pauseindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0xB]);
            ptrs.mcc_pgcrindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0xC]);
            ptrs.mcc_gameindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[0x401C1C0, 0x0]);
            ptrs.mcc_igt_float = DeepPtr::new_64bit(dlls.exe_mcc, &[0x401C204]);
            ptrs.mcc_comptimerstate = DeepPtr::new_64bit(dlls.exe_mcc, &[0x401C1D8, 0x1AC]);

            // Halo 1
            const H1_GLOBALS: u64 = 0x2CA0780;
            const H1_MAP: u64 = 0x2C9F7C4;
            const H1_CINFLAGS: u64 = 0x3005198;
            const H1_COORDS: u64 = 0x2F00954;
            const H1_FADE: u64 = 0x300D678;

            ptrs.h1_tickcounter = DeepPtr::new_64bit(dlls.dll_halo1, &[0x2CEBD34]);
            ptrs.h1_igt = DeepPtr::new_64bit(dlls.dll_halo1, &[0x3008134]);
            ptrs.h1_bspstate = DeepPtr::new_64bit(dlls.dll_halo1, &[0x1CECDFC]);
            ptrs.h1_levelname = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x20]);
            ptrs.h1_gamewon = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_GLOBALS + 0x1]);
            ptrs.h1_cinematic = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_CINFLAGS, 0x0A]);
            ptrs.h1_cutsceneskip = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_CINFLAGS, 0x0B]);
            ptrs.h1_xpos = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_COORDS]);
            ptrs.h1_ypos = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_COORDS + 0x4]);
            ptrs.h1_fadetick = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C0]);
            ptrs.h1_fadelength = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C4]);
            ptrs.h1_fadebyte = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C6]);
            ptrs.h1_deathflag = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_GLOBALS + 0x17]);
            ptrs.h1_checksum = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x64]);
            ptrs.h1_aflags = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x68]);

            // Halo 2
            const H2_CINFLAGS: u64 = 0x15F42B8;
            const H2_COORDS: u64 = 0xE7E308;
            const H2_FADE: u64 = 0xE7E308;

            ptrs.h2_levelname = DeepPtr::new_64bit(dlls.dll_halo2, &[0xE6ED78]);
            ptrs.h2_igt = DeepPtr::new_64bit(dlls.dll_halo2, &[0x15A1BA0]);
            ptrs.h2_bspstate = DeepPtr::new_64bit(dlls.dll_halo2, &[0xDF7D74]);
            ptrs.h2_deathflag = DeepPtr::new_64bit(dlls.dll_halo2, &[0xE7E760, -0xEFi64 as u64]);
            ptrs.h2_tickcounter = DeepPtr::new_64bit(dlls.dll_halo2, &[0x15E1D74]);
            ptrs.h2_graphics = DeepPtr::new_64bit(dlls.dll_halo2, &[0xE1F178]);
            ptrs.h2_fadebyte = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_CINFLAGS, -0x92Ei64 as u64]);
            ptrs.h2_letterbox = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_CINFLAGS, -0x938i64 as u64]);
            ptrs.h2_xpos = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_COORDS]);
            ptrs.h2_ypos = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_COORDS + 0x4]);
            ptrs.h2_fadetick = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_FADE, 0x0]);
            ptrs.h2_fadelength = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_FADE, 0x4]);

            // Halo 3
            ptrs.h3_levelname = DeepPtr::new_64bit(dlls.dll_halo3, &[0x1E92AB8]);
            ptrs.h3_theatertime = DeepPtr::new_64bit(dlls.dll_halo3, &[0x1F2084C]);
            ptrs.h3_tickcounter = DeepPtr::new_64bit(dlls.dll_halo3, &[0x2B34F2C]);
            ptrs.h3_bspstate = DeepPtr::new_64bit(dlls.dll_halo3, &[0xA39220, 0x2C]);
            ptrs.h3_deathflag = DeepPtr::new_64bit(dlls.dll_halo3, &[0x1E19C98, 0xFDCD]);

            // Halo Reach
            ptrs.hr_levelname = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x2A2F6D7]);
            ptrs.hr_bspstate = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x3B9C020]);
            ptrs.hr_deathflag = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x250B808, 0x1ED09]);

            // ODST
            ptrs.odst_levelname = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x20C0DA8]);
            ptrs.odst_streets = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x21463B4]);
            ptrs.odst_bspstate = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x33FD0DC]);
            ptrs.odst_deathflag = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0xFDEAFC, -0x913i64 as u64]);

            // Halo 4
            ptrs.h4_levelname = DeepPtr::new_64bit(dlls.dll_halo4, &[0x2AE485F]);
            ptrs.h4_bspstate = DeepPtr::new_64bit(dlls.dll_halo4, &[0x2746930]);
        }

        3272 => {
            // MCC - Steam/WinStore
            let menustate: u64 = if is_winstore { 0x3E4C034 } else { 0x3FFDAA4 };
            ptrs.mcc_loadindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate]);
            ptrs.mcc_menuindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0x11]);
            ptrs.mcc_pauseindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0xB]);
            ptrs.mcc_pgcrindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0xC]);
            if is_winstore {
                ptrs.mcc_gameindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3E4CA78, 0x0]);
                ptrs.mcc_igt_float = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3E4CB30]);
                ptrs.mcc_comptimerstate = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3E4CA90, 0x1AC]);
            } else {
                ptrs.mcc_gameindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3FFE4D8, 0x0]);
                ptrs.mcc_igt_float = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3FFE590]);
                ptrs.mcc_comptimerstate = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3FFE4F0, 0x1AC]);
            }

            // Halo 1
            const H1_GLOBALS: u64 = 0x2B23700;
            const H1_MAP: u64 = 0x2B22744;
            const H1_CINFLAGS: u64 = 0x2EA01F8;
            const H1_COORDS: u64 = 0x2D9B9C4;
            const H1_FADE: u64 = 0x2EA8708;

            ptrs.h1_tickcounter = DeepPtr::new_64bit(dlls.dll_halo1, &[0x2B6F5E4]);
            ptrs.h1_igt = DeepPtr::new_64bit(dlls.dll_halo1, &[0x2EA31C4]);
            ptrs.h1_bspstate = DeepPtr::new_64bit(dlls.dll_halo1, &[0x1B860A4]);
            ptrs.h1_levelname = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x20]);
            ptrs.h1_gamewon = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_GLOBALS + 0x1]);
            ptrs.h1_cinematic = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_CINFLAGS, 0x0A]);
            ptrs.h1_cutsceneskip = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_CINFLAGS, 0x0B]);
            ptrs.h1_xpos = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_COORDS]);
            ptrs.h1_ypos = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_COORDS + 0x4]);
            ptrs.h1_fadetick = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C0]);
            ptrs.h1_fadelength = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C4]);
            ptrs.h1_fadebyte = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C6]);
            ptrs.h1_deathflag = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_GLOBALS + 0x17]);
            ptrs.h1_checksum = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x64]);
            ptrs.h1_aflags = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x68]);

            // Halo 2
            const H2_CINFLAGS: u64 = 0x15F5788;
            const H2_COORDS: u64 = 0xE7F5E8;
            const H2_FADE: u64 = 0x15EA778;

            ptrs.h2_levelname = DeepPtr::new_64bit(dlls.dll_halo2, &[0xE6FE68]);
            ptrs.h2_igt = DeepPtr::new_64bit(dlls.dll_halo2, &[0x15A2EA0]);
            ptrs.h2_bspstate = DeepPtr::new_64bit(dlls.dll_halo2, &[0xDF8D74]);
            ptrs.h2_deathflag = DeepPtr::new_64bit(dlls.dll_halo2, &[0xE7FA50, -0xEFi64 as u64]);
            ptrs.h2_tickcounter = DeepPtr::new_64bit(dlls.dll_halo2, &[0x15E3074]);
            ptrs.h2_graphics = DeepPtr::new_64bit(dlls.dll_halo2, &[0xE20278]);
            ptrs.h2_fadebyte = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_CINFLAGS, -0x92Ei64 as u64]);
            ptrs.h2_letterbox = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_CINFLAGS, -0x938i64 as u64]);
            ptrs.h2_xpos = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_COORDS]);
            ptrs.h2_ypos = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_COORDS + 0x4]);
            ptrs.h2_fadetick = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_FADE, 0x0]);
            ptrs.h2_fadelength = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_FADE, 0x4]);

            // Halo 3
            ptrs.h3_levelname = DeepPtr::new_64bit(dlls.dll_halo3, &[0x20A8118]);
            ptrs.h3_theatertime = DeepPtr::new_64bit(dlls.dll_halo3, &[0x2135F70]);
            ptrs.h3_tickcounter = DeepPtr::new_64bit(dlls.dll_halo3, &[0x2D3C04C]);
            ptrs.h3_bspstate = DeepPtr::new_64bit(dlls.dll_halo3, &[0xA4E170, 0x2C]);
            ptrs.h3_deathflag = DeepPtr::new_64bit(dlls.dll_halo3, &[0x202F2D8, 0xFDCD]);

            // Halo Reach
            ptrs.hr_levelname = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x2A1F587]);
            ptrs.hr_bspstate = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x4E2FBA8]);
            ptrs.hr_deathflag = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x24FB708, 0x1ED09]);

            // ODST
            ptrs.odst_levelname = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x20EF128]);
            ptrs.odst_streets = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x21F05F8]);
            ptrs.odst_bspstate = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x46E261C]);
            ptrs.odst_deathflag = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x100CB3C, -0x913i64 as u64]);

            // Halo 4
            ptrs.h4_levelname = DeepPtr::new_64bit(dlls.dll_halo4, &[0x2AFF81F]);
            ptrs.h4_bspstate = DeepPtr::new_64bit(dlls.dll_halo4, &[0x275D550]);
        }

        3385 => {
            // MCC - Steam/WinStore
            let menustate: u64 = if is_winstore { 0x3E4B034 } else { 0x3FFCA94 };
            ptrs.mcc_loadindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate]);
            ptrs.mcc_menuindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0x11]);
            ptrs.mcc_pauseindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0xB]);
            ptrs.mcc_pgcrindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0xC]);
            if is_winstore {
                ptrs.mcc_gameindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3E4BA68, 0x0]);
                ptrs.mcc_igt_float = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3E4BB28]);
                ptrs.mcc_comptimerstate = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3E4BA80, 0x1AC]);
            } else {
                ptrs.mcc_gameindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3FFD4C8, 0x0]);
                ptrs.mcc_igt_float = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3FFD588]);
                ptrs.mcc_comptimerstate = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3FFD4E0, 0x1AC]);
            }

            // Halo 1
            const H1_GLOBALS: u64 = 0x2B23700;
            const H1_MAP: u64 = 0x2B22744;
            const H1_CINFLAGS: u64 = 0x2EA0208;
            const H1_COORDS: u64 = 0x2D9B9C4;
            const H1_FADE: u64 = 0x2EA8718;

            ptrs.h1_tickcounter = DeepPtr::new_64bit(dlls.dll_halo1, &[0x2B6F5E4]);
            ptrs.h1_igt = DeepPtr::new_64bit(dlls.dll_halo1, &[0x2EA31D4]);
            ptrs.h1_bspstate = DeepPtr::new_64bit(dlls.dll_halo1, &[0x1B860A4]);
            ptrs.h1_levelname = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x20]);
            ptrs.h1_gamewon = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_GLOBALS + 0x1]);
            ptrs.h1_cinematic = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_CINFLAGS, 0x0A]);
            ptrs.h1_cutsceneskip = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_CINFLAGS, 0x0B]);
            ptrs.h1_xpos = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_COORDS]);
            ptrs.h1_ypos = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_COORDS + 0x4]);
            ptrs.h1_fadetick = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C0]);
            ptrs.h1_fadelength = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C4]);
            ptrs.h1_fadebyte = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C6]);
            ptrs.h1_deathflag = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_GLOBALS + 0x17]);
            ptrs.h1_checksum = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x64]);
            ptrs.h1_aflags = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x68]);

            // Halo 2
            const H2_CINFLAGS: u64 = 0x15F5788;
            const H2_COORDS: u64 = 0xE7F5E8;
            const H2_FADE: u64 = 0x15EA778;

            ptrs.h2_levelname = DeepPtr::new_64bit(dlls.dll_halo2, &[0xE6FE68]);
            ptrs.h2_igt = DeepPtr::new_64bit(dlls.dll_halo2, &[0x15A2EA0]);
            ptrs.h2_bspstate = DeepPtr::new_64bit(dlls.dll_halo2, &[0xDF8D74]);
            ptrs.h2_deathflag = DeepPtr::new_64bit(dlls.dll_halo2, &[0xE7FA50, -0xEFi64 as u64]);
            ptrs.h2_tickcounter = DeepPtr::new_64bit(dlls.dll_halo2, &[0x15E3074]);
            ptrs.h2_graphics = DeepPtr::new_64bit(dlls.dll_halo2, &[0xE20278]);
            ptrs.h2_fadebyte = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_CINFLAGS, -0x92Ei64 as u64]);
            ptrs.h2_letterbox = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_CINFLAGS, -0x938i64 as u64]);
            ptrs.h2_xpos = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_COORDS]);
            ptrs.h2_ypos = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_COORDS + 0x4]);
            ptrs.h2_fadetick = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_FADE, 0x0]);
            ptrs.h2_fadelength = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_FADE, 0x4]);

            // Halo 3
            ptrs.h3_levelname = DeepPtr::new_64bit(dlls.dll_halo3, &[0x20A8118]);
            ptrs.h3_theatertime = DeepPtr::new_64bit(dlls.dll_halo3, &[0x2135F70]);
            ptrs.h3_tickcounter = DeepPtr::new_64bit(dlls.dll_halo3, &[0x2D3C04C]);
            ptrs.h3_bspstate = DeepPtr::new_64bit(dlls.dll_halo3, &[0xA4E170, 0x2C]);
            ptrs.h3_deathflag = DeepPtr::new_64bit(dlls.dll_halo3, &[0x202F2D8, 0xFDCD]);

            // Halo Reach
            ptrs.hr_levelname = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x2A1F587]);
            ptrs.hr_bspstate = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x4E2FBA8]);
            ptrs.hr_deathflag = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x24FB708, 0x1ED09]);

            // ODST
            ptrs.odst_levelname = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x20EF128]);
            ptrs.odst_streets = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x21F05F8]);
            ptrs.odst_bspstate = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x46E261C]);
            ptrs.odst_deathflag = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x100CB3C, -0x913i64 as u64]);

            // Halo 4
            ptrs.h4_levelname = DeepPtr::new_64bit(dlls.dll_halo4, &[0x2AFF81F]);
            ptrs.h4_bspstate = DeepPtr::new_64bit(dlls.dll_halo4, &[0x275D550]);
        }
        3528 | 3498 | 3495 | _ /* Unknown Version, attempt to use latest */ => {
            // MCC - Steam/WinStore
            let menustate: u64 = if is_winstore { 0x3E4EFE4 } else { 0x4000B8C };
            ptrs.mcc_loadindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate]);
            ptrs.mcc_menuindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0x11]);
            ptrs.mcc_pauseindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0xB]);
            ptrs.mcc_pgcrindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[menustate + 0xC]);
            if is_winstore {
                ptrs.mcc_gameindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3E4FAB8, 0x0]);
                ptrs.mcc_igt_float = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3E4FAA4]);
                ptrs.mcc_comptimerstate = DeepPtr::new_64bit(dlls.exe_mcc, &[0x3E4FAF8, 0x1AC]);
            } else {
                ptrs.mcc_gameindicator = DeepPtr::new_64bit(dlls.exe_mcc, &[0x4001658, 0x0]);
                ptrs.mcc_igt_float = DeepPtr::new_64bit(dlls.exe_mcc, &[0x4001644]);
                ptrs.mcc_comptimerstate = DeepPtr::new_64bit(dlls.exe_mcc, &[0x4001698, 0x1AC]);
            }

            // Halo 1
            const H1_GLOBALS: u64 = 0x2B23700;
            const H1_MAP: u64 = 0x2B22744;
            const H1_CINFLAGS: u64 = 0x2EA0208;
            const H1_COORDS: u64 = 0x2D9B9C4;
            const H1_FADE: u64 = 0x2EA8718;

            ptrs.h1_tickcounter = DeepPtr::new_64bit(dlls.dll_halo1, &[0x2B6F5E4]);
            ptrs.h1_igt = DeepPtr::new_64bit(dlls.dll_halo1, &[0x2EA31D4]);
            ptrs.h1_bspstate = DeepPtr::new_64bit(dlls.dll_halo1, &[0x1B860A4]);
            ptrs.h1_levelname = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x20]);
            ptrs.h1_gamewon = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_GLOBALS + 0x1]);
            ptrs.h1_cinematic = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_CINFLAGS, 0x0A]);
            ptrs.h1_cutsceneskip = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_CINFLAGS, 0x0B]);
            ptrs.h1_xpos = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_COORDS]);
            ptrs.h1_ypos = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_COORDS + 0x4]);
            ptrs.h1_fadetick = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C0]);
            ptrs.h1_fadelength = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C4]);
            ptrs.h1_fadebyte = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_FADE, 0x3C6]);
            ptrs.h1_deathflag = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_GLOBALS + 0x17]);
            ptrs.h1_checksum = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x64]);
            ptrs.h1_aflags = DeepPtr::new_64bit(dlls.dll_halo1, &[H1_MAP + 0x68]);

            // Halo 2
            const H2_CINFLAGS: u64 = 0x15F6788;
            const H2_COORDS: u64 = 0xE805E8;
            const H2_FADE: u64 = 0x15EB778;

            ptrs.h2_levelname = DeepPtr::new_64bit(dlls.dll_halo2, &[0xE70E68]);
            ptrs.h2_igt = DeepPtr::new_64bit(dlls.dll_halo2, &[0x15A3EA0]);
            ptrs.h2_bspstate = DeepPtr::new_64bit(dlls.dll_halo2, &[0xDF9D74]);
            ptrs.h2_deathflag = DeepPtr::new_64bit(dlls.dll_halo2, &[0xE80A50, -0xEFi64 as u64]);
            ptrs.h2_tickcounter = DeepPtr::new_64bit(dlls.dll_halo2, &[0x15E4074]);
            ptrs.h2_graphics = DeepPtr::new_64bit(dlls.dll_halo2, &[0xE21278]);
            ptrs.h2_fadebyte = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_CINFLAGS, -0x92Ei64 as u64]);
            ptrs.h2_letterbox = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_CINFLAGS, -0x938i64 as u64]);
            ptrs.h2_xpos = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_COORDS]);
            ptrs.h2_ypos = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_COORDS + 0x4]);
            ptrs.h2_fadetick = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_FADE, 0x0]);
            ptrs.h2_fadelength = DeepPtr::new_64bit(dlls.dll_halo2, &[H2_FADE, 0x4]);

            // Halo 3
            ptrs.h3_levelname = DeepPtr::new_64bit(dlls.dll_halo3, &[0x20A9118]);
            ptrs.h3_theatertime = DeepPtr::new_64bit(dlls.dll_halo3, &[0x2136F70]);
            ptrs.h3_tickcounter = DeepPtr::new_64bit(dlls.dll_halo3, &[0x2D3D04C]);
            ptrs.h3_bspstate = DeepPtr::new_64bit(dlls.dll_halo3, &[0xA4F170, 0x2C]);
            ptrs.h3_deathflag = DeepPtr::new_64bit(dlls.dll_halo3, &[0x20302D8, 0xFDCD]);

            // Halo Reach
            ptrs.hr_levelname = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x2A1F527]);
            ptrs.hr_bspstate = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x4E2FB28]);
            ptrs.hr_deathflag = DeepPtr::new_64bit(dlls.dll_halo_reach, &[0x24FB5F0, 0x1ED09]);

            // ODST
            ptrs.odst_levelname = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x20EF128]);
            ptrs.odst_streets = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x21F05F8]);
            ptrs.odst_bspstate = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x46E261C]);
            ptrs.odst_deathflag = DeepPtr::new_64bit(dlls.dll_halo3_odst, &[0x100CB3C, -0x913i64 as u64]);

            // Halo 4
            ptrs.h4_levelname = DeepPtr::new_64bit(dlls.dll_halo4, &[0x2AFF89F]);
            ptrs.h4_bspstate = DeepPtr::new_64bit(dlls.dll_halo4, &[0x275D5D0]);
        }
    }
}

trait SetTimerVar {
    fn set_timer_var(&self, name: &str);
}

impl SetTimerVar for Watcher<u8> {
    fn set_timer_var(&self, name: &str) {
        match &self.pair {
            Some(pair) => asr::timer::set_variable_int(name, pair.current),
            None => asr::timer::set_variable(name, ""),
        }
    }
}

impl SetTimerVar for Watcher<u16> {
    fn set_timer_var(&self, name: &str) {
        match &self.pair {
            Some(pair) => asr::timer::set_variable_int(name, pair.current),
            None => asr::timer::set_variable(name, ""),
        }
    }
}

impl SetTimerVar for Watcher<u32> {
    fn set_timer_var(&self, name: &str) {
        match &self.pair {
            Some(pair) => asr::timer::set_variable_int(name, pair.current),
            None => asr::timer::set_variable(name, ""),
        }
    }
}

impl SetTimerVar for Watcher<u64> {
    fn set_timer_var(&self, name: &str) {
        match &self.pair {
            Some(pair) => asr::timer::set_variable_int(name, pair.current),
            None => asr::timer::set_variable(name, ""),
        }
    }
}

impl SetTimerVar for Watcher<MCCGame> {
    fn set_timer_var(&self, name: &str) {
        match &self.pair {
            Some(pair) => asr::timer::set_variable(name, &pair.current.to_string()),
            None => asr::timer::set_variable(name, ""),
        }
    }
}

impl SetTimerVar for Watcher<f32> {
    fn set_timer_var(&self, name: &str) {
        match &self.pair {
            Some(pair) => asr::timer::set_variable_float(name, pair.current),
            None => asr::timer::set_variable(name, ""),
        }
    }
}

impl SetTimerVar for Watcher<bool> {
    fn set_timer_var(&self, name: &str) {
        match &self.pair {
            Some(pair) => asr::timer::set_variable(name, if pair.current { "true" } else { "false" }),
            None => asr::timer::set_variable(name, ""),
        }
    }
}

impl<const N: usize> SetTimerVar for Watcher<ArrayCString<N>> {
    fn set_timer_var(&self, name: &str) {
        match &self.pair {
            Some(pair) => asr::timer::set_variable(name, pair.current.validate_utf8().unwrap_or_default()),
            None => asr::timer::set_variable(name, ""),
        }
    }
}

fn update_game_state_all(state: &mut GameState, process: &Process, pointers: &GamePointers) {
    // MCC
    state.mcc_loadindicator.update(pointers.mcc_loadindicator.deref(&process).ok());
    state.mcc_menuindicator.update(pointers.mcc_menuindicator.deref(&process).ok());
    state.mcc_pauseindicator.update(pointers.mcc_pauseindicator.deref(&process).ok());
    state.mcc_pgcrindicator.update(pointers.mcc_pgcrindicator.deref(&process).ok());
    state.mcc_gameindicator.update(pointers.mcc_gameindicator.deref(&process).ok());
    state.mcc_igt_float.update(pointers.mcc_igt_float.deref(&process).ok());
    state.mcc_comptimerstate.update(pointers.mcc_comptimerstate.deref(&process).ok());

    // Halo 1
    state.h1_tickcounter.update(pointers.h1_tickcounter.deref(&process).ok());
    state.h1_igt.update(pointers.h1_igt.deref(&process).ok());
    state.h1_bspstate.update(pointers.h1_bspstate.deref(&process).ok());
    state.h1_levelname.update(pointers.h1_levelname.deref(&process).ok());
    state.h1_gamewon.update(pointers.h1_gamewon.deref(&process).ok());
    state.h1_cinematic.update(pointers.h1_cinematic.deref(&process).ok());
    state.h1_cutsceneskip.update(pointers.h1_cutsceneskip.deref(&process).ok());
    state.h1_xpos.update(pointers.h1_xpos.deref(&process).ok());
    state.h1_ypos.update(pointers.h1_ypos.deref(&process).ok());
    state.h1_fadetick.update(pointers.h1_fadetick.deref(&process).ok());
    state.h1_fadelength.update(pointers.h1_fadelength.deref(&process).ok());
    state.h1_fadebyte.update(pointers.h1_fadebyte.deref(&process).ok());
    state.h1_deathflag.update(pointers.h1_deathflag.deref(&process).ok());
    state.h1_checksum.update(pointers.h1_checksum.deref(&process).ok());
    state.h1_aflags.update(pointers.h1_aflags.deref(&process).ok());

    // Halo 2
    state.h2_levelname.update(pointers.h2_levelname.deref(&process).ok());
    state.h2_igt.update(pointers.h2_igt.deref(&process).ok());
    state.h2_bspstate.update(pointers.h2_bspstate.deref(&process).ok());
    state.h2_deathflag.update(pointers.h2_deathflag.deref(&process).ok());
    state.h2_tickcounter.update(pointers.h2_tickcounter.deref(&process).ok());
    state.h2_graphics.update(pointers.h2_graphics.deref(&process).ok());
    state.h2_fadebyte.update(pointers.h2_fadebyte.deref(&process).ok());
    state.h2_letterbox.update(pointers.h2_letterbox.deref(&process).ok());
    state.h2_xpos.update(pointers.h2_xpos.deref(&process).ok());
    state.h2_ypos.update(pointers.h2_ypos.deref(&process).ok());
    state.h2_fadetick.update(pointers.h2_fadetick.deref(&process).ok());
    state.h2_fadelength.update(pointers.h2_fadelength.deref(&process).ok());

    // Halo 3
    state.h3_levelname.update(pointers.h3_levelname.deref(&process).ok());
    state.h3_theatertime.update(pointers.h3_theatertime.deref(&process).ok());
    state.h3_tickcounter.update(pointers.h3_tickcounter.deref(&process).ok());
    state.h3_bspstate.update(pointers.h3_bspstate.deref(&process).ok());
    state.h3_deathflag.update(pointers.h3_deathflag.deref(&process).ok());

    // Halo Reach
    state.hr_levelname.update(pointers.hr_levelname.deref(&process).ok());
    state.hr_bspstate.update(pointers.hr_bspstate.deref(&process).ok());
    state.hr_deathflag.update(pointers.hr_deathflag.deref(&process).ok());

    // ODST
    state.odst_levelname.update(pointers.odst_levelname.deref(&process).ok());
    state.odst_streets.update(pointers.odst_streets.deref(&process).ok());
    state.odst_bspstate.update(pointers.odst_bspstate.deref(&process).ok());
    state.odst_deathflag.update(pointers.odst_deathflag.deref(&process).ok());

    // Halo 4
    state.h4_levelname.update(pointers.h4_levelname.deref(&process).ok());
    state.h4_bspstate.update(pointers.h4_bspstate.deref(&process).ok());

    // Debug variables - MCC
    state.mcc_loadindicator.set_timer_var("MCC Load Indicator");
    state.mcc_menuindicator.set_timer_var("MCC Menu Indicator");
    state.mcc_pauseindicator.set_timer_var("MCC Pause Indicator");
    state.mcc_pgcrindicator.set_timer_var("MCC PGCR Indicator");
    state.mcc_gameindicator.set_timer_var("MCC Game Indicator");
    state.mcc_igt_float.set_timer_var("MCC IGT Float");
    state.mcc_comptimerstate.set_timer_var("MCC Comp Timer State");

    // Debug variables - Halo 1
    state.h1_tickcounter.set_timer_var("H1 Tick Counter");
    state.h1_igt.set_timer_var("H1 IGT");
    state.h1_bspstate.set_timer_var("H1 BSP State");
    state.h1_levelname.set_timer_var("H1 Level Name");
    state.h1_gamewon.set_timer_var("H1 Game Won");
    state.h1_cinematic.set_timer_var("H1 Cinematic");
    state.h1_cutsceneskip.set_timer_var("H1 Cutscene Skip");
    state.h1_xpos.set_timer_var("H1 X Pos");
    state.h1_ypos.set_timer_var("H1 Y Pos");
    state.h1_fadetick.set_timer_var("H1 Fade Tick");
    state.h1_fadelength.set_timer_var("H1 Fade Length");
    state.h1_fadebyte.set_timer_var("H1 Fade Byte");
    state.h1_deathflag.set_timer_var("H1 Death Flag");
    state.h1_checksum.set_timer_var("H1 Checksum");
    state.h1_aflags.set_timer_var("H1 A Flags");

    // Debug variables - Halo 2
    state.h2_levelname.set_timer_var("H2 Level Name");
    state.h2_igt.set_timer_var("H2 IGT");
    state.h2_bspstate.set_timer_var("H2 BSP State");
    state.h2_deathflag.set_timer_var("H2 Death Flag");
    state.h2_tickcounter.set_timer_var("H2 Tick Counter");
    state.h2_graphics.set_timer_var("H2 Graphics");
    state.h2_fadebyte.set_timer_var("H2 Fade Byte");
    state.h2_letterbox.set_timer_var("H2 Letterbox");
    state.h2_xpos.set_timer_var("H2 X Pos");
    state.h2_ypos.set_timer_var("H2 Y Pos");
    state.h2_fadetick.set_timer_var("H2 Fade Tick");
    state.h2_fadelength.set_timer_var("H2 Fade Length");

    // Debug variables - Halo 3
    state.h3_levelname.set_timer_var("H3 Level Name");
    state.h3_theatertime.set_timer_var("H3 Theater Time");
    state.h3_tickcounter.set_timer_var("H3 Tick Counter");
    state.h3_bspstate.set_timer_var("H3 BSP State");
    state.h3_deathflag.set_timer_var("H3 Death Flag");

    // Debug variables - Halo Reach
    state.hr_levelname.set_timer_var("HR Level Name");
    state.hr_bspstate.set_timer_var("HR BSP State");
    state.hr_deathflag.set_timer_var("HR Death Flag");

    // Debug variables - ODST
    state.odst_levelname.set_timer_var("ODST Level Name");
    state.odst_streets.set_timer_var("ODST Streets");
    state.odst_bspstate.set_timer_var("ODST BSP State");
    state.odst_deathflag.set_timer_var("ODST Death Flag");

    // Debug variables - Halo 4
    state.h4_levelname.set_timer_var("H4 Level Name");
    state.h4_bspstate.set_timer_var("H4 BSP State");
}

async fn main() {
    let mut settings = Settings::register();
    let mut state = GameState::default();
    let mut splitter = SplitterState::default();

    loop {
        let (process, exe_name) = asr::future::retry(|| {
            ["MCC-Win64-Shipping.exe", "MCC-Win64-Shipping-WinStore.exe", "MCCWinStore-Win64-Shipping.exe"]
                .into_iter()
                .find_map(|name| Process::attach(name).map(|p| (p, name)))
        })
        .await;

        let is_winstore = exe_name != "MCC-Win64-Shipping.exe";
        let mcc_addr = process.get_module_address(exe_name).unwrap();
        let mcc_version = FileVersion::read(&process, mcc_addr).unwrap();
        let mcc_version_str = &format!(
            "{}.{}.{}.{}",
            mcc_version.major_version, mcc_version.minor_version, mcc_version.build_part, mcc_version.private_part
        );

        asr::timer::set_variable("MCC Version", mcc_version_str);
        asr::timer::set_variable(
            "Is WinStore",
            match is_winstore {
                true => "true",
                false => "false",
            },
        );

        // WinStore is unsupported below version 3272
        if is_winstore && mcc_version.minor_version < 3272 {
            // We don't want to burn CPU by constantly attaching/detaching so
            // spin in an idle loop until the game is closed.

            print_message(&format!("WinStore version {} is not supported, going into idle mode.", mcc_version_str));

            process
                .until_closes(async {
                    asr::future::next_tick().await;
                })
                .await;

            continue;
        }

        process
            .until_closes(async {
                let mut dlls = GameDLLs::default();
                let mut ptrs = GamePointers::default();

                dlls.exe_mcc = mcc_addr;

                loop {
                    settings.update();

                    dlls.dll_halo1 = process.get_module_address("halo1.dll").unwrap_or_default();
                    dlls.dll_halo2 = process.get_module_address("halo2.dll").unwrap_or_default();
                    dlls.dll_halo3 = process.get_module_address("halo3.dll").unwrap_or_default();
                    dlls.dll_halo4 = process.get_module_address("halo4.dll").unwrap_or_default();
                    dlls.dll_halo3_odst = process.get_module_address("halo3odst.dll").unwrap_or_default();
                    dlls.dll_halo_reach = process.get_module_address("haloreach.dll").unwrap_or_default();

                    asr::timer::set_variable("dll_h1", dlls.dll_halo1.to_string().as_str());
                    asr::timer::set_variable("dll_h2", dlls.dll_halo2.to_string().as_str());
                    asr::timer::set_variable("dll_h3", dlls.dll_halo3.to_string().as_str());
                    asr::timer::set_variable("dll_h4", dlls.dll_halo4.to_string().as_str());
                    asr::timer::set_variable("dll_h3_odst", dlls.dll_halo3_odst.to_string().as_str());
                    asr::timer::set_variable("dll_reach", dlls.dll_halo_reach.to_string().as_str());

                    update_game_pointers(is_winstore, mcc_version, &dlls, &mut ptrs);

                    update_game_state_all(&mut state, &process, &ptrs);
                    update_splitter_state(&mut state, &settings, &mut splitter);

                    // Get current game
                    let current_game = current!(state.mcc_gameindicator).unwrap_or(MCCGame::MainMenu).into();
                    let menu_indicator = current!(state.mcc_menuindicator).unwrap_or(0);
                    let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);

                    // Handle timer state
                    let timer_state = asr::timer::state();

                    match timer_state {
                        TimerState::NotRunning => {
                            if splitter.vars_reset {
                                splitter = SplitterState::default();
                                splitter.vars_reset = false;
                            }

                            // Check for start conditions
                            if should_start(&state, &settings, &mut splitter, current_game, menu_indicator) {
                                asr::timer::start();
                                //splitter.vars_reset = true;
                            }
                        }
                        TimerState::Running | TimerState::Paused => {
                            if !splitter.vars_reset {
                                splitter.vars_reset = true;
                            }

                            // Check for reset
                            if should_reset(&state, &settings, &splitter, current_game, menu_indicator) {
                                print_message("reset");
                                asr::timer::reset();
                                splitter.reset();
                                continue;
                            }

                            // Check for split
                            if should_split(&state, &settings, &mut splitter, current_game, menu_indicator) {
                                asr::timer::split();
                            }

                            // Handle loading/game time
                            handle_loading(&state, &settings, &mut splitter, current_game, menu_indicator, load_indicator);

                            // Update death counter
                            if settings.death_counter {
                                update_death_counter(&state, &mut splitter, current_game);
                            }
                        }
                        TimerState::Ended => {
                            // Timer has ended, wait for reset
                        }
                        _ => {}
                    }

                    // Go around again
                    asr::future::next_tick().await;
                }
            })
            .await;
    }
}

fn update_splitter_state(state: &mut GameState, settings: &Settings, splitter: &mut SplitterState) {
    let menu_indicator = current!(state.mcc_menuindicator).unwrap_or(0);

    if menu_indicator == 0 {
        if splitter.h3_reset_flag || settings.il_mode || settings.any_level {
            splitter.h3_reset_flag = false;
        }
        if splitter.pgcr_exists {
            splitter.pgcr_exists = false;
        }
    }

    let current_game: u8 = current!(state.mcc_gameindicator).unwrap_or(MCCGame::MainMenu).into();

    if current_game == 1 && menu_indicator == 1 {
        update_h2_tgj_flag(state, splitter);
    }
    if current_game == 2 && menu_indicator == 1 && !settings.il_mode && !settings.any_level {
        update_h3_reset_flag(state, splitter)
    }
}

fn update_h2_tgj_flag(state: &GameState, splitter: &mut SplitterState) {
    let level = current!(state.h2_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
    let bspstate = current!(state.h2_bspstate).unwrap_or(255);
    let tickcounter = current!(state.h2_tickcounter).unwrap_or(0);

    if level == "08b" && !splitter.h2_tgj_ready_flag {
        if bspstate == 3 {
            splitter.h2_tgj_ready_flag = true;
            splitter.h2_tgj_ready_time = tickcounter;
        }
    }

    // Reset flag on level change
    let level_old = old!(state.h2_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
    if level != level_old {
        splitter.h2_tgj_ready_flag = false;
        splitter.h2_tgj_ready_time = 0;
    }
}

fn update_h3_reset_flag(state: &GameState, splitter: &mut SplitterState) {
    let level = current!(state.h3_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
    let theatertime = current!(state.h3_theatertime).unwrap_or(0);

    if level == "010" && theatertime >= 15 {
        splitter.h3_reset_flag = true;
    }
}

fn should_start(state: &GameState, settings: &Settings, splitter: &mut SplitterState, current_game: u8, menu_indicator: u8) -> bool {
    if menu_indicator != 1 || splitter.vars_reset {
        return false;
    }

    splitter.started_game = current_game;

    match current_game {
        0 => should_start_h1(state, settings, splitter),
        1 => should_start_h2(state, settings, splitter),
        2 => should_start_h3(state, settings, splitter),
        3 => should_start_h4(state, settings, splitter),
        5 => should_start_odst(state, settings, splitter),
        6 => should_start_hr(state, settings, splitter),
        _ => false,
    }
}

fn should_start_h1(state: &GameState, settings: &Settings, splitter: &mut SplitterState) -> bool {
    let level = match current!(state.h1_levelname) {
        Some(l) => l,
        None => return false,
    };
    let level_str = level.validate_utf8().unwrap_or_default();

    if level_str.is_empty() {
        return false;
    }

    let bspstate = current!(state.h1_bspstate).unwrap_or(255);
    let xpos = current!(state.h1_xpos).unwrap_or(0.0);
    let tickcounter = current!(state.h1_tickcounter).unwrap_or(0);
    let cinematic = current!(state.h1_cinematic).unwrap_or(false);
    let cinematic_old = old!(state.h1_cinematic).unwrap_or(false);
    let cutsceneskip = current!(state.h1_cutsceneskip).unwrap_or(false);
    let cutsceneskip_old = old!(state.h1_cutsceneskip).unwrap_or(false);

    // Check IL start conditions
    let should_start = match level_str {
        "a10" => {
            if settings.il_mode || settings.any_level || level_str == "a10" {
                bspstate == 0 && xpos < -55.0 && tickcounter > 280 && !cinematic && cinematic_old
            } else {
                false
            }
        }
        "a30" => {
            if settings.il_mode || settings.any_level {
                ((tickcounter >= 182 && tickcounter < 190) || (!cinematic && cinematic_old && tickcounter > 500 && tickcounter < 900)) && !cutsceneskip
            } else {
                false
            }
        }
        "a50" => {
            if settings.il_mode || settings.any_level {
                tickcounter > 30 && tickcounter < 900 && !cinematic && cinematic_old
            } else {
                false
            }
        }
        "b30" => {
            if settings.il_mode || settings.any_level {
                tickcounter > 30 && tickcounter < 1060 && !cinematic && cinematic_old
            } else {
                false
            }
        }
        "b40" => {
            if settings.il_mode || settings.any_level {
                tickcounter > 30 && tickcounter < 950 && !cinematic && cinematic_old
            } else {
                false
            }
        }
        "c10" => {
            if settings.il_mode || settings.any_level {
                tickcounter > 30 && tickcounter < 700 && !cinematic && cinematic_old
            } else {
                false
            }
        }
        "c20" | "c40" | "d20" | "d40" => {
            if settings.il_mode || settings.any_level {
                !cutsceneskip && cutsceneskip_old
            } else {
                false
            }
        }
        _ => {
            if settings.any_start {
                (!cutsceneskip && cutsceneskip_old) || (tickcounter > 30 && !cinematic && cinematic_old)
            } else {
                false
            }
        }
    };

    if should_start {
        splitter.started_level = level_str.to_string();
        true
    } else {
        false
    }
}

fn should_start_h2(state: &GameState, settings: &Settings, splitter: &mut SplitterState) -> bool {
    let level = match current!(state.h2_levelname) {
        Some(l) => l,
        None => return false,
    };
    let level_str = level.validate_utf8().unwrap_or_default();

    let tickcounter = current!(state.h2_tickcounter).unwrap_or(0);
    let fadebyte = current!(state.h2_fadebyte).unwrap_or(0);
    let fadebyte_old = old!(state.h2_fadebyte).unwrap_or(0);
    let igt = current!(state.h2_igt).unwrap_or(0);
    let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);
    let bspstate = current!(state.h2_bspstate).unwrap_or(255);

    if settings.il_mode && level_str != "01a" {
        if igt > 10 && igt < 30 {
            splitter.started_level = level_str.to_string();
            return true;
        }
    } else {
        if level_str == "01a" && tickcounter >= 26 && tickcounter < 30 {
            splitter.started_level = level_str.to_string();
            return true;
        } else if level_str == "01b" && load_indicator == 0 && fadebyte == 0 && fadebyte_old == 1 && tickcounter < 30 {
            splitter.started_level = level_str.to_string();
            return true;
        } else if (settings.any_level || settings.il_mode) && load_indicator == 0 {
            if level_str == "03a" {
                // Outskirts special logic
                let fadetick = current!(state.h2_fadetick).unwrap_or(0);
                let fadelength = current!(state.h2_fadelength).unwrap_or(0);
                if fadebyte == 1 && bspstate == 0 && tickcounter > 10 && tickcounter < 100 {
                    if fadelength > 15 && tickcounter >= fadetick + (fadelength as f64 * 0.067) as u32 {
                        splitter.started_level = level_str.to_string();
                        return true;
                    }
                }
            } else if fadebyte == 0 && fadebyte_old == 1 && tickcounter < 120 {
                splitter.started_level = level_str.to_string();
                return true;
            }
        }
    }

    false
}

fn should_start_h3(state: &GameState, settings: &Settings, splitter: &mut SplitterState) -> bool {
    let level = match current!(state.h3_levelname) {
        Some(l) => l,
        None => return false,
    };
    let level_str = level.validate_utf8().unwrap_or_default();

    let igt_float = current!(state.mcc_igt_float).unwrap_or(0.0);
    let theatertime = current!(state.h3_theatertime).unwrap_or(0);
    let tickcounter = current!(state.h3_tickcounter).unwrap_or(0);
    let tickcounter_old = old!(state.h3_tickcounter).unwrap_or(0);
    let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);

    if settings.il_mode {
        if igt_float > 0.167 && igt_float < 0.5 {
            splitter.started_level = level_str.to_string();
            return true;
        }
    } else if settings.any_level || level_str == "010" {
        if load_indicator == 0 && theatertime > 15 && theatertime < 30 {
            splitter.started_level = level_str.to_string();
            return true;
        } else if splitter.h3_reset_flag && level_str == "010" && tickcounter > 0 && tickcounter < 15 && tickcounter > tickcounter_old {
            splitter.started_level = level_str.to_string();
            return true;
        }
    }

    false
}

fn should_start_h4(state: &GameState, settings: &Settings, splitter: &mut SplitterState) -> bool {
    let level = match current!(state.h4_levelname) {
        Some(l) => l,
        None => return false,
    };
    let level_str = level.validate_utf8().unwrap_or_default();

    let igt_float = current!(state.mcc_igt_float).unwrap_or(0.0);

    if (settings.il_mode || settings.any_level || level_str == "m10") && igt_float > 0.167 && igt_float < 0.5 {
        splitter.started_level = level_str.to_string();
        return true;
    }

    false
}

fn should_start_odst(state: &GameState, settings: &Settings, splitter: &mut SplitterState) -> bool {
    let level = match current!(state.odst_levelname) {
        Some(l) => l,
        None => return false,
    };
    let level_str = level.validate_utf8().unwrap_or_default();
    let streets = current!(state.odst_streets).unwrap_or(0);
    let igt_float = current!(state.mcc_igt_float).unwrap_or(0.0);

    if (settings.il_mode || settings.any_level || (level_str == "h100" && streets == 0)) && igt_float > 0.167 && igt_float < 0.5 {
        splitter.started_level = level_str.to_string();
        splitter.started_scene = streets;
        return true;
    }

    false
}

fn should_start_hr(state: &GameState, settings: &Settings, splitter: &mut SplitterState) -> bool {
    let level = match current!(state.hr_levelname) {
        Some(l) => l,
        None => return false,
    };
    let level_str = level.validate_utf8().unwrap_or_default();
    let igt_float = current!(state.mcc_igt_float).unwrap_or(0.0);

    if (settings.il_mode || settings.any_level || level_str == "m10") && igt_float > 0.167 && igt_float < 0.5 {
        splitter.started_level = level_str.to_string();
        return true;
    }

    false
}

fn should_reset(state: &GameState, settings: &Settings, splitter: &SplitterState, current_game: u8, menu_indicator: u8) -> bool {
    if settings.loop_mode {
        return false;
    }

    // Reset on main menu in IL mode
    if settings.il_mode && menu_indicator == 0 && asr::timer::state() != TimerState::Ended {
        return true;
    }

    if menu_indicator != 1 {
        return false;
    }

    match current_game {
        0 => should_reset_h1(state, settings, splitter),
        1 => should_reset_h2(state, settings, splitter),
        2 => should_reset_h3(state, settings, splitter),
        3 => should_reset_h4(state, settings, splitter),
        5 => should_reset_odst(state, settings, splitter),
        6 => should_reset_hr(state, settings, splitter),
        _ => false,
    }
}

fn should_reset_h1(state: &GameState, settings: &Settings, splitter: &SplitterState) -> bool {
    if splitter.started_game != 0 || asr::timer::state() == TimerState::Ended {
        return false;
    }

    let level = current!(state.h1_levelname).unwrap_or_default();
    let igt = current!(state.h1_igt).unwrap_or(0);
    let igt_old = old!(state.h1_igt).unwrap_or(0);
    let tickcounter = current!(state.h1_tickcounter).unwrap_or(0);
    let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);
    let load_indicator_old = old!(state.mcc_loadindicator).unwrap_or(0);

    let target_level = if settings.il_mode || settings.any_level {
        splitter.started_level.as_str()
    } else {
        "a10"
    };

    if level.validate_utf8().unwrap_or_default() == target_level {
        return (igt < igt_old && igt < 10) || (load_indicator == 0 && load_indicator_old == 1 && tickcounter < 60);
    }

    false
}

fn should_reset_h2(state: &GameState, settings: &Settings, splitter: &SplitterState) -> bool {
    if splitter.started_game != 1 || asr::timer::state() == TimerState::Ended {
        return false;
    }

    let level = current!(state.h2_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
    let igt = current!(state.h2_igt).unwrap_or(0);
    let igt_old = old!(state.h2_igt).unwrap_or(0);
    let tickcounter = current!(state.h2_tickcounter).unwrap_or(0);
    let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);
    let load_indicator_old = old!(state.mcc_loadindicator).unwrap_or(0);

    if settings.il_mode || settings.any_level {
        if level == splitter.started_level.as_str() {
            return (igt < igt_old && igt < 10) || (load_indicator == 1 && igt == 0);
        }
    } else {
        if level == "01a" || (level == "01b" && splitter.started_level.as_str() != "01a") || level == "00a" {
            return (igt < igt_old && igt < 10) || (load_indicator == 0 && load_indicator_old == 1 && tickcounter < 60);
        }
    }

    false
}

fn should_reset_h3(state: &GameState, settings: &Settings, splitter: &SplitterState) -> bool {
    if splitter.started_game != 2 || asr::timer::state() == TimerState::Ended {
        return false;
    }

    let level = current!(state.h3_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
    let igt_float = current!(state.mcc_igt_float).unwrap_or(0.0);
    let igt_float_old = old!(state.mcc_igt_float).unwrap_or(0.0);
    let theatertime = current!(state.h3_theatertime).unwrap_or(0);
    let tickcounter = current!(state.h3_tickcounter).unwrap_or(0);
    let tickcounter_old = old!(state.h3_tickcounter).unwrap_or(0);
    let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);
    let load_indicator_old = old!(state.mcc_loadindicator).unwrap_or(0);

    if settings.il_mode {
        return level == splitter.started_level.as_str() && igt_float < igt_float_old && igt_float < 0.167;
    } else {
        if settings.any_level {
            return level == splitter.started_level.as_str() && theatertime > 0 && theatertime < 15;
        } else if level == "005" {
            return load_indicator == 0 && load_indicator_old == 1 && tickcounter < 60;
        } else if level == "010" {
            return (theatertime > 0 && theatertime < 15) || (theatertime >= 15 && tickcounter < tickcounter_old && tickcounter < 10 && load_indicator == 0);
        }
    }

    false
}

fn should_reset_h4(state: &GameState, settings: &Settings, splitter: &SplitterState) -> bool {
    if splitter.started_game != 3 || asr::timer::state() == TimerState::Ended {
        return false;
    }

    let level = current!(state.h4_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
    let igt_float = current!(state.mcc_igt_float).unwrap_or(0.0);
    let igt_float_old = old!(state.mcc_igt_float).unwrap_or(0.0);
    let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);

    let target_level = if settings.il_mode || settings.any_level {
        splitter.started_level.as_str()
    } else {
        "m10"
    };

    if level == target_level {
        return (igt_float < igt_float_old && igt_float < 0.167) || (load_indicator == 1 && igt_float == 0.0);
    }

    false
}

fn should_reset_odst(state: &GameState, settings: &Settings, splitter: &SplitterState) -> bool {
    if splitter.started_game != 5 || asr::timer::state() == TimerState::Ended {
        return false;
    }

    let level = current!(state.odst_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
    let streets = current!(state.odst_streets).unwrap_or(0);
    let igt_float = current!(state.mcc_igt_float).unwrap_or(0.0);
    let igt_float_old = old!(state.mcc_igt_float).unwrap_or(0.0);
    let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);

    if settings.any_level || settings.il_mode {
        if level == splitter.started_level.as_str() && splitter.started_scene == streets {
            return (igt_float < igt_float_old && igt_float < 0.167) || (load_indicator == 1 && igt_float == 0.0);
        }
    } else {
        if (level == "c100" || (level == "h100" && streets == 0)) {
            return (igt_float < igt_float_old && igt_float < 0.167) || (load_indicator == 1 && igt_float == 0.0);
        }
    }

    false
}

fn should_reset_hr(state: &GameState, settings: &Settings, splitter: &SplitterState) -> bool {
    if splitter.started_game != 6 || asr::timer::state() == TimerState::Ended {
        return false;
    }

    let level = current!(state.hr_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
    let igt_float = current!(state.mcc_igt_float).unwrap_or(0.0);
    let igt_float_old = old!(state.mcc_igt_float).unwrap_or(0.0);
    let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);

    let target_level = if settings.il_mode || settings.any_level {
        splitter.started_level.as_str()
    } else {
        "m10"
    };

    if level == target_level {
        return (igt_float < igt_float_old && igt_float < 0.167) || (load_indicator == 1 && igt_float == 0.0);
    }

    false
}

fn should_split(state: &GameState, settings: &Settings, splitter: &mut SplitterState, current_game: u8, menu_indicator: u8) -> bool {
    // Force split for sq_split
    if splitter.force_split2 {
        splitter.force_split2 = false;
        splitter.clear_dirty_bsps();
        return true;
    }

    if menu_indicator != 1 {
        return false;
    }

    // Force split from IGT/RTA logic
    if splitter.force_split {
        splitter.force_split = false;
        splitter.clear_dirty_bsps();
        if settings.loop_mode {
            splitter.loop_split = false;
        }
        return true;
    }

    if splitter.multigame_pause {
        return false;
    }

    match current_game {
        0 => should_split_h1(state, settings, splitter),
        1 => should_split_h2(state, settings, splitter),
        2 => should_split_h3(state, settings, splitter),
        3 => should_split_h4(state, settings, splitter),
        5 => should_split_odst(state, settings, splitter),
        6 => should_split_hr(state, settings, splitter),
        _ => false,
    }
}

fn should_split_h1(state: &GameState, settings: &Settings, splitter: &mut SplitterState) -> bool {
    let level = current!(state.h1_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
    let bspstate = current!(state.h1_bspstate).unwrap_or(255);
    let bspstate_old = old!(state.h1_bspstate).unwrap_or(255);
    let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);
    let load_indicator_old = old!(state.mcc_loadindicator).unwrap_or(0);

    // BSP mode splitting
    if settings.bsp_mode && bspstate != bspstate_old {
        let bsp_list = get_h1_bsp_list(&level);
        if bsp_list.contains(&bspstate) {
            if settings.bsp_cache || !splitter.contains_dirty_bsp_byte(bspstate) {
                // Special handling for b40 and c40
                if level == "b40" && bspstate == 0 {
                    let ypos = current!(state.h1_ypos).unwrap_or(0.0);
                    if ypos > -19.544 && ypos < -19.144 {
                        if !settings.bsp_cache {
                            splitter.add_dirty_bsp_byte(bspstate);
                        }
                        return true;
                    }
                    return false;
                } else if level == "c40" && bspstate == 0 {
                    let xpos = current!(state.h1_xpos).unwrap_or(0.0);
                    let ypos = current!(state.h1_ypos).unwrap_or(0.0);
                    if xpos > 171.87326 && xpos < 185.818526 && ypos > -295.3629 && ypos < -284.356986 {
                        if !settings.bsp_cache {
                            splitter.add_dirty_bsp_byte(bspstate);
                        }
                        return true;
                    }
                    return false;
                } else {
                    if !settings.bsp_cache {
                        splitter.add_dirty_bsp_byte(bspstate);
                    }
                    return true;
                }
            }
        }
    }

    // IL end splits
    if settings.il_mode && !settings.igt_mode {
        let cinematic = current!(state.h1_cinematic).unwrap_or(false);
        let cinematic_old = old!(state.h1_cinematic).unwrap_or(false);
        let cutsceneskip = current!(state.h1_cutsceneskip).unwrap_or(false);
        let cutsceneskip_old = old!(state.h1_cutsceneskip).unwrap_or(false);
        let fadelength = current!(state.h1_fadelength).unwrap_or(0);
        let fadebyte = current!(state.h1_fadebyte).unwrap_or(0);
        let xpos = current!(state.h1_xpos).unwrap_or(0.0);
        let deathflag = current!(state.h1_deathflag).unwrap_or(false);
        let tickcounter = current!(state.h1_tickcounter).unwrap_or(0);

        let should_split = match level.as_str() {
            "a10" => bspstate == 6 && !cutsceneskip_old && cutsceneskip,
            "a30" => bspstate == 1 && !cutsceneskip_old && cutsceneskip,
            "a50" => (bspstate == 3 || bspstate == 2) && !cutsceneskip_old && cutsceneskip && fadelength == 15,
            "b30" => bspstate == 0 && !cinematic && !cutsceneskip_old && cutsceneskip,
            "b40" => bspstate == 2 && !cutsceneskip_old && cutsceneskip,
            "c10" => bspstate != 2 && !cutsceneskip_old && cutsceneskip,
            "c20" => cinematic && !cinematic_old && tickcounter > 30,
            "c40" => tickcounter > 30 && !cutsceneskip_old && cutsceneskip && fadebyte != 1,
            "d20" => fadelength == 30 && !cinematic_old && cinematic,
            "d40" => !cinematic_old && cinematic && !cutsceneskip && xpos > 1000.0 && !deathflag,
            _ => false,
        };

        if should_split {
            splitter.clear_dirty_bsps();
            if settings.loop_mode {
                splitter.loading = true;
            }
            return true;
        }
    }

    // Full game split on loading screen
    if !settings.il_mode && !settings.igt_mode {
        if load_indicator == 1 && load_indicator_old == 0 {
            splitter.clear_dirty_bsps();
            return true;
        }
    }

    false
}

fn should_split_h2(state: &GameState, settings: &Settings, splitter: &mut SplitterState) -> bool {
    let level = current!(state.h2_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
    let bspstate = current!(state.h2_bspstate).unwrap_or(255);
    let bspstate_old = old!(state.h2_bspstate).unwrap_or(255);
    let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);
    let load_indicator_old = old!(state.mcc_loadindicator).unwrap_or(0);

    // BSP mode
    if settings.bsp_mode && bspstate != bspstate_old {
        if settings.bsp_cache {
            let bsp_list = get_h2_bsp_list(&level);
            if bsp_list.contains(&bspstate) {
                return true;
            }
        } else {
            // Special TGJ handling
            if level == "08b" {
                return should_split_h2_tgj(state, splitter);
            }

            // Other level-specific handling
            match level.as_str() {
                "01b" => {
                    let bsp_list = get_h2_bsp_list(&level);
                    if bsp_list.contains(&bspstate) && !splitter.contains_dirty_bsp_byte(bspstate) {
                        if bspstate == 0 && !splitter.contains_dirty_bsp_byte(2) {
                            return false;
                        }
                        splitter.add_dirty_bsp_byte(bspstate);
                        return true;
                    }
                }
                "04a" => {
                    let bsp_list = get_h2_bsp_list(&level);
                    if bsp_list.contains(&bspstate) && !splitter.contains_dirty_bsp_byte(bspstate) {
                        if bspstate == 0 && !splitter.contains_dirty_bsp_byte(3) {
                            return false;
                        }
                        splitter.add_dirty_bsp_byte(bspstate);
                        return true;
                    }
                }
                "04b" => {
                    if bspstate == 3 && !splitter.contains_dirty_bsp_byte(3) {
                        splitter.add_dirty_bsp_byte(3);
                    }
                    let bsp_list = get_h2_bsp_list(&level);
                    if bsp_list.contains(&bspstate) && !splitter.contains_dirty_bsp_byte(bspstate) {
                        if bspstate == 0 && splitter.contains_dirty_bsp_byte(3) {
                            return true;
                        }
                        splitter.add_dirty_bsp_byte(bspstate);
                        return true;
                    }
                }
                "08a" => {
                    let bsp_list = get_h2_bsp_list(&level);
                    if bsp_list.contains(&bspstate) && !splitter.contains_dirty_bsp_byte(bspstate) {
                        if bspstate == 0 && !splitter.contains_dirty_bsp_byte(1) {
                            return false;
                        }
                        splitter.add_dirty_bsp_byte(bspstate);
                        return true;
                    }
                }
                _ => {
                    let bsp_list = get_h2_bsp_list(&level);
                    if bsp_list.contains(&bspstate) && !splitter.contains_dirty_bsp_byte(bspstate) {
                        splitter.add_dirty_bsp_byte(bspstate);
                        return true;
                    }
                }
            }
        }
    }

    // Full game split
    if !(settings.il_mode || settings.igt_mode) {
        if load_indicator == 1 && load_indicator_old == 0 && level != "00a" {
            splitter.clear_dirty_bsps();
            return true;
        }
    }

    false
}

fn should_split_h2_tgj(state: &GameState, splitter: &mut SplitterState) -> bool {
    let bspstate = current!(state.h2_bspstate).unwrap_or(255);
    let bspstate_old = old!(state.h2_bspstate).unwrap_or(255);

    if bspstate == bspstate_old {
        return false;
    }

    let xpos = current!(state.h2_xpos).unwrap_or(0.0);
    let ypos = current!(state.h2_ypos).unwrap_or(0.0);

    match bspstate {
        1 => {
            // First transition to BSP 1: near start
            if !splitter.contains_dirty_bsp_byte(1) && xpos > -2.0 && xpos < 5.0 && ypos > -35.0 && ypos < -15.0 {
                splitter.add_dirty_bsp_byte(1);
                return true;
            }
            // Third transition to BSP 1: after BSP 10
            else if !splitter.contains_dirty_bsp_byte(21) && splitter.contains_dirty_bsp_byte(10) && xpos > 15.0 && xpos < 25.0 && ypos > 15.0 && ypos < 30.0
            {
                splitter.add_dirty_bsp_byte(21);
                return true;
            }
        }
        0 => {
            // Second transition to BSP 0
            if !splitter.contains_dirty_bsp_byte(10) && xpos > -20.0 && xpos < -10.0 && ypos > 20.0 && ypos < 30.0 {
                splitter.add_dirty_bsp_byte(10);
                return true;
            }
            // Fourth transition to BSP 0: after BSP 21
            else if !splitter.contains_dirty_bsp_byte(20) && splitter.contains_dirty_bsp_byte(21) && xpos > 45.0 && xpos < 55.0 && ypos > -5.0 && ypos < 10.0
            {
                splitter.add_dirty_bsp_byte(20);
                return true;
            }
        }
        3 => {
            if !splitter.contains_dirty_bsp_byte(3) {
                splitter.add_dirty_bsp_byte(3);
                return true;
            }
        }
        _ => {}
    }

    false
}

fn should_split_h3(state: &GameState, settings: &Settings, splitter: &mut SplitterState) -> bool {
    let level = current!(state.h3_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
    let bspstate = current!(state.h3_bspstate).unwrap_or(0);
    let bspstate_old = old!(state.h3_bspstate).unwrap_or(0);
    let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);
    let load_indicator_old = old!(state.mcc_loadindicator).unwrap_or(0);

    // BSP mode
    if settings.bsp_mode && bspstate != bspstate_old {
        let bsp_list = get_h3_bsp_list(&level);

        if settings.bsp_cache {
            if bsp_list.contains(&bspstate) {
                return true;
            }
        } else {
            if bsp_list.contains(&bspstate) && !splitter.contains_dirty_bsp_long(bspstate) {
                splitter.add_dirty_bsp_long(bspstate);
                return true;
            }
        }
    }

    // Full game split
    if !settings.il_mode {
        if load_indicator == 1 && load_indicator_old == 0 {
            splitter.clear_dirty_bsps();
            return true;
        }
    }

    false
}

fn should_split_h4(state: &GameState, settings: &Settings, splitter: &mut SplitterState) -> bool {
    let level = current!(state.h4_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
    let bspstate = current!(state.h4_bspstate).unwrap_or(0);
    let bspstate_old = old!(state.h4_bspstate).unwrap_or(0);
    let comptimerstate = current!(state.mcc_comptimerstate).unwrap_or(0);
    let comptimerstate_old = old!(state.mcc_comptimerstate).unwrap_or(0);
    let igt_float = current!(state.mcc_igt_float).unwrap_or(0.0);
    let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);
    let pgcr_indicator = current!(state.mcc_pgcrindicator).unwrap_or(0);

    if settings.comp_splits {
        if load_indicator == 0 && pgcr_indicator == 0 && comptimerstate != comptimerstate_old && comptimerstate != 0 && igt_float > 2.0 {
            return true;
        }
    } else if settings.bsp_mode && bspstate != bspstate_old {
        let bsp_list = get_h4_bsp_list(&level);

        // H4 uses inverted check - split if NOT in list
        if settings.bsp_cache {
            if !bsp_list.contains(&bspstate) {
                return true;
            }
        } else {
            if !bsp_list.contains(&bspstate) && !splitter.contains_dirty_bsp_long(bspstate) {
                splitter.add_dirty_bsp_long(bspstate);
                return true;
            }
        }
    }

    false
}

fn should_split_odst(state: &GameState, settings: &Settings, splitter: &mut SplitterState) -> bool {
    let level = current!(state.odst_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
    let bspstate = current!(state.odst_bspstate).unwrap_or(0);
    let bspstate_old = old!(state.odst_bspstate).unwrap_or(0);
    let comptimerstate = current!(state.mcc_comptimerstate).unwrap_or(0);
    let comptimerstate_old = old!(state.mcc_comptimerstate).unwrap_or(0);
    let igt_float = current!(state.mcc_igt_float).unwrap_or(0.0);
    let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);
    let pgcr_indicator = current!(state.mcc_pgcrindicator).unwrap_or(0);

    if settings.comp_splits {
        let invalid_state = if level == "l300" { 876414390 } else { 0 };
        if load_indicator == 0
            && pgcr_indicator == 0
            && comptimerstate != comptimerstate_old
            && comptimerstate != invalid_state
            && comptimerstate != 0
            && igt_float > 2.0
        {
            return true;
        }
    } else if settings.bsp_mode && bspstate != bspstate_old {
        let bsp_list = get_odst_bsp_list(&level);

        if igt_float > 0.5 {
            if settings.bsp_cache {
                if bsp_list.contains(&bspstate) {
                    return true;
                }
            } else {
                if bsp_list.contains(&bspstate) && !splitter.contains_dirty_bsp_int(bspstate) {
                    splitter.add_dirty_bsp_int(bspstate);
                    return true;
                }
            }
        }
    }

    false
}

fn should_split_hr(state: &GameState, settings: &Settings, splitter: &mut SplitterState) -> bool {
    let level = current!(state.hr_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
    let bspstate = current!(state.hr_bspstate).unwrap_or(0);
    let bspstate_old = old!(state.hr_bspstate).unwrap_or(0);

    if settings.bsp_mode && bspstate != bspstate_old {
        let bsp_list = get_hr_bsp_list(&level);

        if settings.bsp_cache {
            if bsp_list.contains(&bspstate) {
                return true;
            }
        } else {
            if bsp_list.contains(&bspstate) && !splitter.contains_dirty_bsp_int(bspstate) {
                splitter.add_dirty_bsp_int(bspstate);
                return true;
            }
        }
    }

    false
}

fn handle_loading(state: &GameState, settings: &Settings, splitter: &mut SplitterState, current_game: u8, menu_indicator: u8, load_indicator: u8) {
    // Check for multigame pause/resume
    if !splitter.multigame_pause && !settings.il_mode {
        if check_multigame_pause(state, settings, splitter, current_game) {
            splitter.multigame_pause = true;

            // Store current game time for multigame (we track it ourselves)
            splitter.multigame_time = splitter.game_time;

            // Set force split for end of game
            if current_game == 0 || current_game == 1 {
                splitter.force_split = true;
            }

            // Reset TGJ flag after H2
            if current_game == 1 {
                splitter.h2_tgj_ready_flag = false;
            }
        }
    } else if splitter.multigame_pause {
        if check_multigame_resume(state, splitter, current_game) {
            splitter.multigame_pause = false;
        }
    }

    // Pause timer logic
    let should_pause = splitter.multigame_pause || (settings.menu_pause && (load_indicator == 1 || menu_indicator == 0)) || splitter.loading;

    if should_pause {
        asr::timer::pause_game_time();
    } else {
        asr::timer::resume_game_time();
    }

    // Handle RTA load removal for H1 and H2
    if !splitter.multigame_pause {
        match current_game {
            0 if !settings.igt_mode && !settings.il_mode => {
                handle_h1_loading(state, splitter, load_indicator);
            }
            1 if !settings.igt_mode && !settings.il_mode => {
                handle_h2_loading(state, splitter, load_indicator);
            }
            _ => {}
        }
    }

    // Update game time for IGT-based games
    if menu_indicator == 1 && !splitter.multigame_pause {
        update_game_time(state, settings, splitter, current_game);
    }
}

fn update_game_time(state: &GameState, settings: &Settings, splitter: &mut SplitterState, current_game: u8) {
    // Only handle IGT for games that use it (H3, H4, ODST, Reach) or when igt_mode is on
    let uses_igt = settings.igt_mode || matches!(current_game, 2 | 3 | 5 | 6);

    // Also handle H1/H2 in IL mode
    let is_rta_game = matches!(current_game, 0 | 1) && !settings.il_mode && !settings.igt_mode;

    if !uses_igt && !is_rta_game {
        return;
    }

    let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);
    let pgcr_indicator = current!(state.mcc_pgcrindicator).unwrap_or(0);
    let pgcr_indicator_old = old!(state.mcc_pgcrindicator).unwrap_or(0);
    let load_indicator_old = old!(state.mcc_loadindicator).unwrap_or(0);

    // Get IGT and tickrate based on game
    let (igt, igt_old, tickrate): (u32, u32, u8) = match current_game {
        0 => {
            let igt = current!(state.h1_igt).unwrap_or(0);
            let igt_old = old!(state.h1_igt).unwrap_or(0);
            (igt, igt_old, 30)
        }
        1 => {
            let igt = current!(state.h2_igt).unwrap_or(0);
            let igt_old = old!(state.h2_igt).unwrap_or(0);
            (igt, igt_old, 60)
        }
        2 => {
            if settings.il_mode {
                let igt = (current!(state.mcc_igt_float).unwrap_or(0.0) * 60.0).round() as u32;
                let igt_old = (old!(state.mcc_igt_float).unwrap_or(0.0) * 60.0).round() as u32;
                (igt, igt_old, 60)
            } else {
                let igt = current!(state.h3_theatertime).unwrap_or(0);
                let igt_old = old!(state.h3_theatertime).unwrap_or(0);
                (igt, igt_old, 60)
            }
        }
        3 | 5 | 6 => {
            let igt = (current!(state.mcc_igt_float).unwrap_or(0.0) * 60.0).round() as u32;
            let igt_old = (old!(state.mcc_igt_float).unwrap_or(0.0) * 60.0).round() as u32;
            (igt, igt_old, 60)
        }
        _ => return,
    };

    // Track level time
    if splitter.level_time == 0 {
        if load_indicator == 0 && !splitter.pgcr_exists {
            splitter.level_time = igt;
        }
    } else if igt > igt_old && (igt - igt_old) < 300 {
        splitter.level_time += igt - igt_old;
    }

    // Handle PGCR (level complete) or loading screen
    if pgcr_indicator == 1 && pgcr_indicator_old == 0 {
        let rounded = splitter.level_time - (splitter.level_time % tickrate as u32);
        splitter.ingame_time += rounded;
        splitter.level_time = 0;
        splitter.pgcr_exists = true;
        splitter.force_split = true;
    } else if load_indicator == 1 && load_indicator_old == 0 {
        if !splitter.pgcr_exists {
            let rounded = splitter.level_time - (splitter.level_time % tickrate as u32);
            splitter.ingame_time += rounded;
            splitter.level_time = 0;
            splitter.force_split = true;
        }
        splitter.pgcr_exists = false;
    } else if igt < igt_old && igt < 10 && load_indicator == 0 {
        // Level restart
        let rounded = if settings.igt_add {
            splitter.level_time
        } else if splitter.level_time < tickrate as u32 && igt_old < tickrate as u32 {
            tickrate as u32
        } else if (splitter.level_time % tickrate as u32) > (tickrate as u32 / 2) {
            splitter.level_time + (tickrate as u32 - (splitter.level_time % tickrate as u32))
        } else {
            splitter.level_time - (splitter.level_time % tickrate as u32)
        };
        splitter.ingame_time += rounded;
        splitter.level_time = 0;
    }

    // Calculate and set game time
    let total_ticks = if load_indicator == 1 {
        splitter.ingame_time
    } else {
        splitter.ingame_time + splitter.level_time
    };

    let ms = (1000.0 / tickrate as f64) * total_ticks as f64;
    splitter.game_time = asr::time::Duration::milliseconds(ms as i64) + splitter.multigame_time;

    asr::timer::set_game_time(splitter.game_time);
}

fn handle_h1_loading(state: &GameState, splitter: &mut SplitterState, load_indicator: u8) {
    let menu_indicator = current!(state.mcc_menuindicator).unwrap_or(0);
    let load_indicator_old = old!(state.mcc_loadindicator).unwrap_or(0);
    let gamewon = current!(state.h1_gamewon).unwrap_or(false);
    let gamewon_old = old!(state.h1_gamewon).unwrap_or(false);
    let tickcounter = current!(state.h1_tickcounter).unwrap_or(0);
    let tickcounter_old = old!(state.h1_tickcounter).unwrap_or(0);
    if !splitter.loading {
        if menu_indicator == 1 {
            if gamewon && !gamewon_old {
                splitter.loading = true;
            }
        } else if load_indicator == 1 && load_indicator_old == 0 {
            splitter.loading = true;
        }
    } else {
        if tickcounter == tickcounter_old + 1 {
            splitter.loading = false;
        }
    }
}
fn handle_h2_loading(state: &GameState, splitter: &mut SplitterState, load_indicator: u8) {
    // Simplified H2 loading logic
    let menu_indicator = current!(state.mcc_menuindicator).unwrap_or(0);
    let load_indicator_old = old!(state.mcc_loadindicator).unwrap_or(0);
    let fadebyte = current!(state.h2_fadebyte).unwrap_or(0);
    let fadebyte_old = old!(state.h2_fadebyte).unwrap_or(0);
    let letterbox = current!(state.h2_letterbox).unwrap_or(0.0);
    let letterbox_old = old!(state.h2_letterbox).unwrap_or(0.0);
    let tickcounter = current!(state.h2_tickcounter).unwrap_or(0);
    let bspstate = current!(state.h2_bspstate).unwrap_or(255);
    let pause_indicator = current!(state.mcc_pauseindicator).unwrap_or(0);

    if !splitter.loading {
        if menu_indicator == 1 {
            // Check for level end fade
            if (tickcounter > 60 && fadebyte == 1 && fadebyte_old == 1 && letterbox > 0.96 && letterbox_old <= 0.96 && letterbox_old != 0.0)
                || load_indicator == 1
            {
                splitter.loading = true;
            }
        } else if load_indicator == 1 && load_indicator_old == 0 {
            splitter.loading = true;
        }
    } else {
        if menu_indicator == 1 && load_indicator == 0 {
            if fadebyte == 0 && fadebyte_old == 1 && pause_indicator == 0 && bspstate != 255 {
                splitter.loading = false;
            } else if fadebyte == 0 && tickcounter > 10 && bspstate != 255 {
                splitter.loading = false;
            }
        }
    }
}

fn check_multigame_pause(state: &GameState, settings: &Settings, splitter: &mut SplitterState, current_game: u8) -> bool {
    if settings.il_mode || settings.any_level {
        return false;
    }

    match current_game {
        0 => {
            // H1 - PoA ending
            let level = current!(state.h1_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
            let cinematic = current!(state.h1_cinematic).unwrap_or(false);
            let cinematic_old = old!(state.h1_cinematic).unwrap_or(false);
            let cutsceneskip = current!(state.h1_cutsceneskip).unwrap_or(false);
            let xpos = current!(state.h1_xpos).unwrap_or(0.0);
            let deathflag = current!(state.h1_deathflag).unwrap_or(false);

            level == "d40" && !cinematic_old && cinematic && !cutsceneskip && xpos > 1000.0 && !deathflag
        }
        1 => {
            // H2 - TGJ ending
            let level = current!(state.h2_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
            let fadebyte = current!(state.h2_fadebyte).unwrap_or(0);
            let letterbox = current!(state.h2_letterbox).unwrap_or(0.0);
            let letterbox_old = old!(state.h2_letterbox).unwrap_or(0.0);
            let tickcounter = current!(state.h2_tickcounter).unwrap_or(0);

            level == "08b"
                && fadebyte == 1
                && letterbox > 0.96
                && letterbox_old <= 0.96
                && letterbox_old != 0.0
                && splitter.h2_tgj_ready_flag
                && tickcounter > (splitter.h2_tgj_ready_time + 300)
        }
        2 => {
            // H3 - Halo ending
            let level = current!(state.h3_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
            let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);
            let load_indicator_old = old!(state.mcc_loadindicator).unwrap_or(0);

            load_indicator == 1 && load_indicator_old == 0 && level == "130"
        }
        3 => {
            // H4 - Midnight ending
            let level = current!(state.h4_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
            let pgcr = current!(state.mcc_pgcrindicator).unwrap_or(0);
            let pgcr_old = old!(state.mcc_pgcrindicator).unwrap_or(0);

            pgcr == 1 && pgcr_old == 0 && level == "m90"
        }
        5 => {
            // ODST - Coastal ending
            let level = current!(state.odst_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
            let pgcr = current!(state.mcc_pgcrindicator).unwrap_or(0);
            let pgcr_old = old!(state.mcc_pgcrindicator).unwrap_or(0);

            pgcr == 1 && pgcr_old == 0 && level == "l300"
        }
        6 => {
            // Reach - PoA ending
            let level = current!(state.hr_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
            let pgcr = current!(state.mcc_pgcrindicator).unwrap_or(0);
            let pgcr_old = old!(state.mcc_pgcrindicator).unwrap_or(0);

            pgcr == 1 && pgcr_old == 0 && level == "m70"
        }
        _ => false,
    }
}

fn check_multigame_resume(state: &GameState, splitter: &SplitterState, current_game: u8) -> bool {
    match current_game {
        0 => {
            // H1 - PoA start
            let level = current!(state.h1_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
            let bspstate = current!(state.h1_bspstate).unwrap_or(255);
            let xpos = current!(state.h1_xpos).unwrap_or(0.0);
            let tickcounter = current!(state.h1_tickcounter).unwrap_or(0);
            let cinematic = current!(state.h1_cinematic).unwrap_or(false);
            let cinematic_old = old!(state.h1_cinematic).unwrap_or(false);

            level == "a10" && bspstate == 0 && xpos < -55.0 && tickcounter > 280 && !cinematic && cinematic_old
        }
        1 => {
            // H2 - Armory/Cairo start
            let level = current!(state.h2_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
            let tickcounter = current!(state.h2_tickcounter).unwrap_or(0);
            let fadebyte = current!(state.h2_fadebyte).unwrap_or(0);
            let fadebyte_old = old!(state.h2_fadebyte).unwrap_or(0);
            let load_indicator = current!(state.mcc_loadindicator).unwrap_or(0);

            (level == "01a" && tickcounter >= 26 && tickcounter < 30)
                || (level == "01b" && load_indicator == 0 && fadebyte == 0 && fadebyte_old == 1 && tickcounter < 30)
        }
        2 => {
            // H3 - Sierra start
            let level = current!(state.h3_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
            let theatertime = current!(state.h3_theatertime).unwrap_or(0);

            level == "010" && theatertime > 15 && theatertime < 30
        }
        3 => {
            // H4 - Dawn start
            let level = current!(state.h4_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
            let igt_float = current!(state.mcc_igt_float).unwrap_or(0.0);

            level == "m10" && igt_float > 0.167 && igt_float < 0.5
        }
        5 => {
            // ODST - Mombasa Streets start
            let level = current!(state.odst_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
            let streets = current!(state.odst_streets).unwrap_or(0);
            let igt_float = current!(state.mcc_igt_float).unwrap_or(0.0);

            level == "h100" && streets == 0 && igt_float > 0.167 && igt_float < 0.5
        }
        6 => {
            // Reach - Winter Contingency start
            let level = current!(state.hr_levelname).unwrap_or_default().validate_utf8().unwrap_or("").to_string();
            let igt_float = current!(state.mcc_igt_float).unwrap_or(0.0);

            level == "m10" && igt_float > 0.167 && igt_float < 0.5
        }
        _ => false,
    }
}

fn update_death_counter(state: &GameState, splitter: &mut SplitterState, current_game: u8) {
    let died = match current_game {
        0 => changed_to!(state.h1_deathflag, true),
        1 => changed_to!(state.h2_deathflag, true),
        2 => changed_to!(state.h3_deathflag, true),
        5 => changed_to!(state.odst_deathflag, true),
        6 => changed_to!(state.hr_deathflag, true),
        _ => false,
    };
    if died {
        splitter.death_counter += 1;
        asr::timer::set_variable_int("Deaths", splitter.death_counter);
    }
}
