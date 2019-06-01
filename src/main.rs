use warp::{Filter, path};
use display_as::{HTML, display};
use serde_derive::{Deserialize, Serialize};

fn main() {
    let style_css = path!("style.css").and(warp::fs::file("style.css"));
    let style_css_2 = path!("style.css").and(warp::fs::file("style.css"));

    warp::serve(style_css
                .or(style_css_2))
        .run(([0, 0, 0, 0], 3000));
}

#[derive(Debug, Hash, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Thing {
    name: String,
    description: String,
    times_used: u64,
    times_skipped: u64,
    children: Vec<Thing>,
}

impl Thing {
    fn priority(&self) -> u64 {
        self.times_used + self.times_skipped
    }
}

impl PartialOrd for Thing {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Thing {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority().cmp(&other.priority())
    }
}
