mod proto;

pub use self::proto::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proto_usage() {
        let mut request = Request::new();
        assert!(!request.has_get());
        assert!(!request.has_set());
        assert!(!request.has_delete());
        assert!(!request.has_scan());

        let mut get = request::Get::new();
        get.set_key(String::from("hello"));
        request.set_get(get);
        assert_eq!(request.get_get().get_key(), "hello");
        assert!(request.has_get());
        assert!(!request.has_set());
        assert!(!request.has_delete());
        assert!(!request.has_scan());

        request.kind = Some(Request_oneof_kind::delete(request::Delete::new()));
        assert!(!request.has_get());
        assert!(!request.has_set());
        assert!(request.has_delete());
        assert!(!request.has_scan());

        request.kind = None;
        assert!(!request.has_get());
        assert!(!request.has_set());
        assert!(!request.has_delete());
        assert!(!request.has_scan());
    }
}
