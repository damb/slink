use time::PrimitiveDateTime;

pub fn time_as_seedlink_str(t: &PrimitiveDateTime) -> String {
    format!(
        "{},{:02},{:02},{:02},{:02},{:02}",
        t.year(),
        t.month() as u8,
        t.day(),
        t.hour(),
        t.minute(),
        t.second(),
    )
}

