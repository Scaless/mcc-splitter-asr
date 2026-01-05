use asr::time::Duration;
use pod_enum::pod_enum;

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[pod_enum]
#[repr(u8)]
pub enum MCCGame {
    Halo1 = 0,
    Halo2 = 1,
    Halo3 = 2,
    Halo4 = 3,
    ODST = 5,
    Reach = 6,
    MainMenu = 10,
}
impl core::fmt::Display for MCCGame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Default, Clone)]
pub struct H1Checklist {
    pub a10: u32,
    pub a30: u32,
    pub a50: u32,
    pub b30: u32,
    pub b40: u32,
    pub c10: u32,
    pub c20: u32,
    pub c40: u32,
    pub d20: u32,
    pub d40: u32,
}

impl H1Checklist {
    fn get(&self, level: &str) -> Option<u32> {
        match level {
            "a10" => Some(self.a10),
            "a30" => Some(self.a30),
            "a50" => Some(self.a50),
            "b30" => Some(self.b30),
            "b40" => Some(self.b40),
            "c10" => Some(self.c10),
            "c20" => Some(self.c20),
            "c40" => Some(self.c40),
            "d20" => Some(self.d20),
            "d40" => Some(self.d40),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct SplitterState {
    // Run tracking
    pub vars_reset: bool,
    pub started_level: String,
    pub level_loaded: String,
    pub started_game: u8,
    pub started_scene: u8,

    // BSP tracking
    pub dirty_bsps_byte: Vec<u8>,
    pub dirty_bsps_int: Vec<u32>,
    pub dirty_bsps_long: Vec<u64>,

    // Split flags
    pub loop_split: bool,
    pub force_split: bool,
    pub force_split2: bool,

    // Halo 2 TGJ
    pub h2_tgj_ready_flag: bool,
    pub h2_tgj_ready_time: u32,

    // Load tracking
    pub last_internal: bool,
    pub old_tick: i32,
    pub loading: bool,

    // Multi-game
    pub multigame_pause: bool,
    pub multigame_time: Duration,

    // IGT tracking
    pub game_time: Duration,
    pub ingame_time: u32,
    pub level_time: u32,
    pub pgcr_exists: bool,

    // H1 validity check
    pub is_valid: bool,
    pub c_time: Duration,
    pub diff: Duration,

    // H3 specific
    pub h3_reset_flag: bool,

    // Death counter
    pub death_counter: u32,
}

impl SplitterState {
    pub fn reset(&mut self) {
        self.dirty_bsps_byte.clear();
        self.dirty_bsps_int.clear();
        self.dirty_bsps_long.clear();

        self.started_level = String::default();
        self.level_loaded = String::default();
        self.started_game = 10;
        self.started_scene = 0;

        self.loop_split = true;
        self.force_split = false;
        self.force_split2 = false;

        self.h2_tgj_ready_flag = false;
        self.h2_tgj_ready_time = 0;
        self.last_internal = false;
        self.old_tick = -2;
        self.loading = false;
        self.multigame_pause = false;
        self.multigame_time = Duration::ZERO;

        self.game_time = Duration::ZERO;
        self.ingame_time = 0;
        self.level_time = 0;
        self.pgcr_exists = false;

        self.is_valid = false;
        self.c_time = Duration::ZERO;
        self.diff = Duration::ZERO;

        self.death_counter = 0;
    }

    pub fn clear_dirty_bsps(&mut self) {
        self.dirty_bsps_byte.clear();
        self.dirty_bsps_int.clear();
        self.dirty_bsps_long.clear();
    }

    pub fn add_dirty_bsp_byte(&mut self, bsp: u8) {
        if !self.dirty_bsps_byte.contains(&bsp) {
            self.dirty_bsps_byte.push(bsp);
        }
    }

    pub fn add_dirty_bsp_int(&mut self, bsp: u32) {
        if !self.dirty_bsps_int.contains(&bsp) {
            self.dirty_bsps_int.push(bsp);
        }
    }

    pub fn add_dirty_bsp_long(&mut self, bsp: u64) {
        if !self.dirty_bsps_long.contains(&bsp) {
            self.dirty_bsps_long.push(bsp);
        }
    }

    pub fn contains_dirty_bsp_byte(&self, bsp: u8) -> bool {
        self.dirty_bsps_byte.contains(&bsp)
    }

    pub fn contains_dirty_bsp_int(&self, bsp: u32) -> bool {
        self.dirty_bsps_int.contains(&bsp)
    }

    pub fn contains_dirty_bsp_long(&self, bsp: u64) -> bool {
        self.dirty_bsps_long.contains(&bsp)
    }
}

pub fn get_h1_bsp_list(level: &str) -> &'static [u8] {
    match level {
        "a10" => &[1, 2, 3, 4, 5, 6],         // PoA
        "a30" => &[1],                        // Halo
        "a50" => &[1, 2, 3],                  // TnR
        "b30" => &[1],                        // SC
        "b40" => &[0, 1, 2, 4, 8, 9, 10, 11], // AotCR
        "c10" => &[1, 3, 4, 5],               // 343
        "c20" => &[1, 2, 3],                  // Library
        "c40" => &[12, 10, 1, 9, 8, 6, 0, 5], // TB
        "d20" => &[4, 3, 2],                  // Keyes
        "d40" => &[1, 2, 3, 4, 5, 6, 7],      // Maw
        _ => &[],
    }
}

pub fn get_h2_bsp_list(level: &str) -> &'static [u8] {
    match level {
        "01a" => &[],              // Armory
        "01b" => &[2, 0, 3],       // Cairo
        "03a" => &[1, 2],          // OS
        "03b" => &[1],             // Metro
        "04a" => &[3, 0],          // Arby
        "04b" => &[0, 2, 1, 5],    // Oracle
        "05a" => &[1],             // DH
        "05b" => &[1, 2],          // Regret
        "06a" => &[1, 2],          // SI
        "06b" => &[1, 2, 3],       // QZ
        "07a" => &[1, 2, 3, 4, 5], // GM
        "08a" => &[1, 0],          // Uprising
        "07b" => &[1, 2, 4],       // HC
        "08b" => &[0, 1, 3],       // TGJ
        _ => &[],
    }
}

pub fn get_h3_bsp_list(level: &str) -> &'static [u64] {
    match level {
        "010" => &[7, 4111, 4127, 8589938751, 12884907135, 4294972543, 4294972927, 6143],
        "020" => &[
            2753726871765283,
            351925325267239,
            527984624664871,
            527980329698111,
            355107896034111,
            495845384389503,
            1058778157941759,
            2081384101315583,
            2076028277097471,
            2043042928264191,
        ],
        "030" => &[708669603847, 1812476198927, 1709396983839, 128849018943, 2327872274495],
        "040" => &[
            70746701299715,
            76347338653703,
            5987184410895,
            43920335569183,
            52712133624127,
            4449586119039,
            110002702385663,
            127560528691711,
        ],
        "050" => &[137438953607, 154618822791, 167503724703, 98784247967, 98784247999, 133143986431, 111669150207],
        "070" => &[
            319187993615142919,
            497073530286903311,
            5109160733019475999,
            7059113264503853119,
            7058267740062093439,
            5296235395170702591,
            6467180094380056063,
            6471685893030682623,
            6453663797939806207,
        ],
        "100" => &[
            4508347378708774919,
            2060429875000377375,
            4384271889560765215,
            2060429875000378143,
            4508347378708775711,
            4229124150272197439,
            4105313024951190527,
            4159567262287660031,
            4153434048988972031,
            4099400491367139327,
            21673629041340192,
        ],
        "110" => &[4294967459, 4294967527, 4294967535, 4294967551],
        "120" => &[1030792151055, 691489734703, 1924145349759, 1133871367679, 1202590844927, 1219770714111],
        _ => &[],
    }
}

pub fn get_h4_bsp_list(level: &str) -> &'static [u64] {
    match level {
        "m10" => &[0, 0x0000000001800000, 0x000000000700000F],
        "m02" => &[0, 0x0000000080000C02],
        "m30" => &[0, 0x0000000072001902],
        "m40" => &[0, 0x00000040000C0001, 0x00000000013C0001],
        "m60" => &[0, 0x0000C00002100001, 0x0000400006000001],
        "m70" => &[0, 0x0000000100100004],
        "m80" => &[0, 0x0020000080000006, 0x0000000080400006, 0x0000000180C0000E],
        "m90" => &[0, 0x0000010000000006, 0x0000000000A00006],
        _ => &[],
    }
}

pub fn get_hr_bsp_list(level: &str) -> &'static [u32] {
    match level {
        "m10" => &[143, 175, 239, 495],
        "m20" => &[249, 505, 509, 511],
        "m30" => &[269, 781, 797, 1821, 1853, 1917],
        "m35" => &[4111, 4127, 4223, 4607, 5119],
        "m45" => &[31, 383, 10111, 12159, 16255, 32639],
        "m50" => &[5135, 5151, 5247, 5631, 8191],
        "m52" => &[],
        "m60" => &[113, 125, 4221, 4223, 5119],
        "m70" => &[31, 63, 127, 255, 511, 1023, 2047],
        _ => &[],
    }
}

pub fn get_odst_bsp_list(level: &str) -> &'static [u32] {
    match level {
        "h100" => &[296, 352, 304, 400, 896, 262, 388, 259],
        "sc10" => &[14, 13, 9],
        "sc11" => &[79, 92, 96],
        "sc13" => &[11, 3, 7],
        "sc12" => &[11, 14, 12],
        "sc14" => &[11, 14, 12],
        "sc15" => &[14, 28, 24],
        "l200" => &[14, 28, 24, 48, 208, 224, 416],
        "l300" => &[33, 41, 56, 112],
        _ => &[],
    }
}
