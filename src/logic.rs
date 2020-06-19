/// Returns the absolute path of a given `relative_url`.
pub fn absolute_path(current_page: &str, relative_url: &str) -> String {
    let parts: Vec<_> = current_page.split('/').collect();

    if relative_url.starts_with("http") {
        relative_url.to_string()
    } else if relative_url.starts_with('/') {
        format!("{}//{}{}", parts[0], parts[2], relative_url)
    } else if relative_url.starts_with("../") {
        let count = relative_url.split("../").count();

        let src = relative_url.replace("../", "");
        let mut url = format!("{}//{}", parts[0], parts[2]);

        if parts.len() != 3 {
            for (i, item) in parts[3..].iter().enumerate() {
                if parts.len() == i + 3 + count {
                    break;
                }
                url = format!("{}/{}", url, item);
            }
        }
        format!("{}/{}", url, src)
    } else {
        let mut url = format!("{}//{}", parts[0], parts[2]);

        if parts.len() != 3 {
            for (i, item) in parts[3..].iter().enumerate() {
                if parts.len() == i + 4 {
                    break;
                }
                url = format!("{}/{}", url, item);
            }
        }
        format!("{}/{}", url, relative_url)
    }
}

#[cfg(test)]
mod get_url {
    use super::*;

    #[test]
    fn absolute() {
        let current_page = "https://example.com/test/index.php";
        let absolute = "https://example.com/test/image.jpg";

        assert_eq!(
            String::from("https://example.com/test/image.jpg"),
            absolute_path(current_page, absolute)
        );
    }

    #[test]
    fn relative_up() {
        let current_page = "https://example.com/test/index.php";
        let relative_up = "../image.jpg";

        assert_eq!(
            String::from("https://example.com/image.jpg"),
            absolute_path(current_page, relative_up)
        );
    }

    #[test]
    fn relative_up_twice() {
        let current_page = "https://example.com/test/lol/index.php";
        let relative_up = "../../image.jpg";

        assert_eq!(
            String::from("https://example.com/image.jpg"),
            absolute_path(current_page, relative_up)
        );
    }

    #[test]
    fn relative_same() {
        let current_page = "https://example.com/test/index.php";
        let relative_same = "image.jpg";

        assert_eq!(
            String::from("https://example.com/test/image.jpg"),
            absolute_path(current_page, relative_same)
        );
    }

    #[test]
    fn root() {
        let current_page = "https://example.com/test/index.php";
        let root = "/lol/image.jpg";

        assert_eq!(
            String::from("https://example.com/lol/image.jpg"),
            absolute_path(current_page, root)
        );
    }
}
