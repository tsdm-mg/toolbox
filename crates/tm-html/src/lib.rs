use select::node::{Data, Node};
use select::predicate::Name;

pub trait HtmlElementExt {
    /// Check if is HTML element node.
    fn is_element(&self) -> bool;

    /// Check if is plain text.
    fn is_text(&self) -> bool;

    /// Return the text in first child if it's a text node.
    fn first_child_text(&self) -> Option<String>;

    /// Parsing a li type node which contains an em node and extra text in it,
    /// returns as a key value pair.
    ///
    /// ```html
    /// <li>
    ///   <em>here_is_key</em>
    ///   here_is_value
    /// </li>
    /// ```
    ///
    /// Note: Both key and value will be trimmed, which means removed white spaces
    /// around themselves.
    ///
    /// Note: Returns the html code if value is an or more Element not text node.
    ///
    /// If any of key or value is null, return null.
    ///
    /// Set `second` to true to force retrieve the second node (text) as value.
    fn parse_li_em_group(&self, second: bool) -> Option<(String, String)>;

    /// Assume current node has an image url like `<img>`, return the url if any.
    fn image_url(&self) -> Option<String>;

    /// Find the deepest text, looks like in-order or post-order traversal but
    /// stops when reach the leaf child.
    /// <a>
    ///   <a>
    ///     <a>
    ///       "1"
    ///     </a>
    ///     <a>
    ///       <a>
    ///         "2"
    ///       </a>
    ///       "3"
    ///     </a>
    /// </a>
    /// Return "1".
    fn first_end_deep_text(&self) -> String;
}

impl<'a> HtmlElementExt for Node<'a> {
    fn is_element(&self) -> bool {
        match self.data() {
            Data::Element(..) => true,
            Data::Text(..) | Data::Comment(..) => false,
        }
    }

    fn is_text(&self) -> bool {
        match self.data() {
            Data::Text(..) => true,
            Data::Element(..) | Data::Comment(..) => false,
        }
    }

    fn first_child_text(&self) -> Option<String> {
        self.first_child().and_then(|x| match x.data() {
            Data::Text(text) => Some(text.to_string()),
            Data::Element(..) | Data::Comment(..) => None,
        })
    }

    fn parse_li_em_group(&self, second: bool) -> Option<(String, String)> {
        // Check if the first child is `<em>`.
        let key = match self.children().filter(|x| x.is_element()).next() {
            Some(v) => match v.data() {
                Data::Element(name, ..) if name.local.to_string() == "em" => {
                    match v.first_child_text() {
                        Some(v) => v,
                        None => return None,
                    }
                }
                _ => return None,
            },
            None => return None,
        };

        let value = if second {
            self.children()
                .skip(1)
                .next()
                .and_then(|x| Some(x.text().trim().to_string()))
        } else if self.children().count() >= 2 && !second {
            // More than one element.
            // Try to remove the first <em> element and return all html code left.
            let v = self
                .children()
                .skip_while(|x| x.is(Name("em")))
                .map(|x| x.html().trim().to_string())
                .collect::<Vec<_>>()
                .join("");
            Some(v)
        } else {
            // Expected value is a text node.
            // Use the trimmed text
            self.children()
                .last()
                .and_then(|x| Some(x.text().trim().to_string()))
        };

        if key.is_empty() || value.is_none() || value.as_ref()?.is_empty() {
            return None;
        }

        Some((key, value.unwrap()))
    }

    fn image_url(&self) -> Option<String> {
        self.attr("zoomfile")
            .or_else(|| self.attr("data-original"))
            .or_else(|| self.attr("src"))
            .or_else(|| self.attr("file"))
            .and_then(|x| Some(x.to_owned()))
    }

    fn first_end_deep_text(&self) -> String {
        let mut ch = self.children().filter(|x| x.is_element());
        if let Some(v) = ch.next() {
            v.first_end_deep_text()
        } else {
            self.text()
        }
    }
}
