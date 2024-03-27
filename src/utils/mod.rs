pub fn is_public_ipv6(ipv6: &str) -> bool {
    let private_prefixes = vec![
        "fd",
        "fc",
        "fe80",
    ];

    for prefix in private_prefixes {
        if ipv6.starts_with(prefix) {
            return false;
        }
    }

    return true;
}
