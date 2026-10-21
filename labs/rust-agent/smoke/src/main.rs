fn main() {
    println!("{}", greeting("climate-agent"));
}

fn greeting(name: &str) -> String {
    format!("hello, {name}")
}

#[cfg(test)]
mod tests {
    use super::greeting;

    #[test]
    fn greets_by_name() {
        assert_eq!(greeting("inmet"), "hello, inmet");
    }
}
