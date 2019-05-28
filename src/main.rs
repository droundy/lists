use warp::{Filter, path};
use display_as::{HTML, display};

fn main() {
    let style_css = path!("style.css").and(warp::fs::file("style.css"));
    let style_css_2 = path!("style.css").and(warp::fs::file("style.css"));

    warp::serve(style_css
                .or(style_css_2))
        .run(([0, 0, 0, 0], 3000));
}
