mod proto;

pub use self::proto::*;

pub fn get_request(key: &str) -> Request {
    let mut request = Request::new();
    let mut get = request::Get::new();
    get.set_key(key.to_string());
    request.set_get(get);
    request
}

pub fn get_response(value: Option<String>) -> Response {
    let mut response = Response::new();
    let mut get = response::Get::new();
    if let Some(value) = value {
        get.set_value(value.to_owned());
        get.set_is_found(true);
    } else {
        get.set_is_found(false);
    }
    response.set_get(get);
    response
}

pub fn set_request(key: &str, value: &str) -> Request {
    let mut request = Request::new();
    let mut set = request::Set::new();
    set.set_key(key.to_string());
    set.set_value(value.to_string());
    request.set_set(set);
    request
}

pub fn set_response() -> Response {
    let mut response = Response::new();
    let mut set = response::Set::new();
    set.set_is_success(true);
    response.set_set(set);
    response
}

pub fn delete_request(key: &str) -> Request {
    let mut request = Request::new();
    let mut delete = request::Delete::new();
    delete.set_key(key.to_string());
    request.set_delete(delete);
    request
}

pub fn delete_response(value: Option<String>) -> Response {
    let mut response = Response::new();
    let mut delete = response::Delete::new();
    if let Some(value) = value {
        delete.set_value(value);
        delete.set_is_found(true);
    } else {
        delete.set_is_found(false);
    }
    response.set_delete(delete);
    response
}

pub fn scan_request() -> Request {
    let mut request = Request::new();
    request.set_scan(request::Scan::new());
    request
}

pub fn scan_response(v: Vec<String>) -> Response {
    let mut response = Response::new();
    let mut scan = response::Scan::new();
    scan.set_keys(v.into_iter().map(|s| s.to_owned()).collect());
    response.set_scan(scan);
    response
}

pub fn ping_request() -> Request {
    let mut request = Request::new();
    request.set_ping(true);
    request
}

pub fn ping_response() -> Response {
    let mut response = Response::new();
    response.set_pong(true);
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_helper_functions() {
        let mut request = Request::new();
        let mut get = request::Get::new();
        get.set_key(String::from("hello"));
        request.set_get(get);

        assert_eq!(get_request("hello"), request);

        let mut set = request::Set::new();
        set.set_key(String::from("hello"));
        set.set_value(String::from("world"));
        request.set_set(set);

        assert_eq!(set_request("hello", "world"), request);

        let mut delete = request::Delete::new();
        delete.set_key(String::from("hello"));
        request.set_delete(delete);

        assert_eq!(delete_request("hello"), request);

        request.set_scan(request::Scan::new());

        assert_eq!(scan_request(), request);
    }

    #[test]
    fn test_proto_usage() {
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
