// Chart-query domain enums, shared only by routes::charts.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Basic,
    Advanced,
    Expert,
    Master,
    ReMaster,
}

impl Difficulty {
    // The code stored in sheets.difficulty / sheet_expr.
    pub fn code(&self) -> &'static str {
        match self {
            Difficulty::Basic => "basic",
            Difficulty::Advanced => "advanced",
            Difficulty::Expert => "expert",
            Difficulty::Master => "master",
            Difficulty::ReMaster => "remaster",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChartType {
    Dx,
    Std,
    Utage,
}

impl ChartType {
    pub fn code(&self) -> &'static str {
        match self {
            ChartType::Dx => "dx",
            ChartType::Std => "std",
            ChartType::Utage => "utage",
        }
    }
}
