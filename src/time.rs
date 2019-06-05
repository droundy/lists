use serde_derive::{Deserialize, Serialize};

pub fn now() -> Time {
    Time(std::time::SystemTime::now()
         .duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs())
}
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Time(u64);
pub static MINUTE: Time = Time(60);
pub static HOUR: Time = Time(60*MINUTE.0);
pub static DAY: Time = Time(24*HOUR.0);

impl std::ops::Sub for Time {
    type Output = Time;

    fn sub(self, other: Time) -> Time {
        Time(self.0 - other.0)
    }
}
impl std::ops::Add for Time {
    type Output = Time;

    fn add(self, other: Time) -> Time {
        Time(self.0 + other.0)
    }
}
impl std::ops::Div<u64> for Time {
    type Output = Time;

    fn div(self, other: u64) -> Time {
        Time(self.0/other)
    }
}
impl std::ops::Mul<u64> for Time {
    type Output = Time;

    fn mul(self, other: u64) -> Time {
        Time(self.0*other)
    }
}
impl std::ops::Mul<Time> for u64 {
    type Output = Time;

    fn mul(self, other: Time) -> Time {
        Time(self*other.0)
    }
}

pub fn geometric_mean(data: &[Time]) -> Time {
    let mut product = 1.0;
    for &d in data.iter() {
        product *= d.0 as f64;
    }
    Time(product.powf(1.0/data.len() as f64) as u64)
}
