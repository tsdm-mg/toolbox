mod bin;

#[cfg(test)]
mod tests {
    use tm_bbcode_macro::bbx;

    const TABLE_WIDTH_30: i32 = 30;
    const TABLE_WIDTH_110: i32 = 110;

    struct Foo {
        foo: i32,
    }

    #[test]
    pub fn test_bbx() {
        let foo = Foo { foo: 1 };

        impl Foo {
            fn get_number() -> i32 {
                100
            }
        }

        let x = bbx!(
            url {
                "x",
                ${ Foo::get_number()}
            },
        );
    }
}
