use kpu::hello;

fn main() {
    hello();
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hello_test() {
        hello();
    }
}
