use display_as::{display, with_template, DisplayAs, HTML, URL, UTF8};
use serde::{Deserialize, Serialize};
use warp::reply::Reply;
use warp::{path, Filter};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Sheet {
    name: String,
}

pub fn sheets() -> impl Filter<Extract=impl warp::Reply, Error=warp::Rejection>+Clone {
    let index = path!("sheets")
        .map(|| {
            println!("I am doing index.");
            display(HTML, &Index).into_response()
        });
    let list = path!("sheets" / String / String).map(|code: String, listname: String| {
        let listname = percent_encoding::percent_decode(listname.as_bytes())
            .decode_utf8()
            .unwrap();
        let code = percent_encoding::percent_decode(code.as_bytes())
            .decode_utf8()
            .unwrap();
        // let x = ThingList::read(&code, &listname);
        display(HTML, &"goodbye").into_response()
    });
    let list_of_lists = path!("sheets" / String).map(|code: String| {
        println!("list of lists: {}", code);
        let code = percent_encoding::percent_decode(code.as_bytes())
            .decode_utf8()
            .unwrap();
        // let x = ThingList::read(&code, "fixme");
        display(HTML, &"hello").into_response()
    });
    list.or(list_of_lists).or(index)
}


struct Index;
#[with_template("[%" "%]" "index.html")]
impl DisplayAs<HTML> for Index {}