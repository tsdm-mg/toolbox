use tm_bbcode_macro::bbx;

#[derive(Debug)]
struct Foo {
    foo: i32,
}

impl Foo {
    fn get_number() -> i32 {
        100
    }
}

fn main() {
    let foo = Foo { foo: 1 };

    let output = bbx!(
        hr { / },
        a {
          { ${foo.foo} }  ,
            "x"
        },
        url {
            {"https://tsdm39.com"},
            ${foo.foo},
        }
    );
    println!("{output}")
}