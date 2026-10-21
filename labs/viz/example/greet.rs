mod shout;

use shout::shout;

pub fn greet(name: &str) -> String {
    shout(&format!("hello, {name}"))
}

fn main() {
    println!("{}", greet("viz"));
}
