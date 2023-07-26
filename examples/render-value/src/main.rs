use minijinja::value::ValueBox;
use minijinja::{context, Environment};

fn main() {
    let env = Environment::new();

    // this just demonstrates that `context!` creates a `ValueBox`
    let ctx: ValueBox = context! {
        name => "Peter"
    };

    // Which can be directly passed to `render_str`.
    println!("{}", env.render_str("Hello {{ name }}!", ctx).unwrap());
}
