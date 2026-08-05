#[derive(Debug, Clone)]
pub struct DayForecast {
    pub day_name: String,      // e.g. "Tuesday" -> translated
    pub date: String,          // e.g. "Aug 5" (numbers kept as-is)
    pub condition: String,     // e.g. "Sunny" -> translated
    pub high_temp: String,     // e.g. "34°" (number kept, unit may translate)
    pub low_temp: String,      // e.g. "22°"
    pub precip_chance: String, // e.g. "10%"
    pub summary: String,       // short RealFeel/description text -> translated
}

impl DayForecast {}
