pub trait Hhmmss {
    fn sms(&self) -> (i64, i64);
    fn hhmmss(&self) -> String {
        let (s, ms) = self.sms();
        s2hhmmss(s)
    }
    fn hhmmssxxx(&self) -> String {
        let (s, ms) = self.sms();
        sms2hhmmsxxx(s, ms)
    }
}

impl Hhmmss for std::time::Duration {
    fn sms(&self) -> (i64, i64) {
        let s = self.as_secs();
        let ms = self.subsec_millis();
        (s as i64, ms as i64)
    }
}

fn s2hhmmss(s: i64) -> String {
    let mut neg = false;
    let mut s = s;
    if s < 0 {
        neg = true;
        s = -s;
    }
    let (h, s) = (s / 3600, s % 3600);
    let (m, s) = (s / 60, s % 60);
    format!("{}{:02}:{:02}:{:02}", if neg { "-" } else { "" }, h, m, s)
}

fn sms2hhmmsxxx(s: i64, ms: i64) -> String {
    let mut neg = false;
    let (mut s, mut ms) = (s, ms);
    if s < 0 {
        neg = true;
        s = -s;
        ms = -ms;
    }
    let (h, s) = (s / 3600, s % 3600);
    let (m, s) = (s / 60, s % 60);
    format!(
        "{}{:02}:{:02}:{:02}.{:03}",
        if neg { "-" } else { "" },
        h,
        m,
        s,
        ms
    )
}
