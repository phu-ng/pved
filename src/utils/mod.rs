pub fn generate_fqdn(name: &str, domain: &str) -> String {
    format!("{}.{}", name, domain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_ipv6() {
        let ipv6_addr = "2001:0db8:85a3:0000:0000:8a2e:0370:7334"; // Global IPv6 address
        let ipv6_addr2 = "fe80::c8e1:31ff:fe9f:bc77"; // Link-local IPv6 address
        let ipv6_addr3 = "::1"; // Loopback IPv6 address
        let ipv6_addr4 = "fc01:abcd::1";

        assert_eq!(is_public_ipv6(ipv6_addr), true);
        assert_eq!(is_public_ipv6(ipv6_addr2), false);
        assert_eq!(is_public_ipv6(ipv6_addr3), false);
        assert_eq!(is_public_ipv6(ipv6_addr4), false);
    }

    #[test]
    fn test_get_ip() {
        println!("{:?}", generate_fqdn("pihole", "phu.homes."))
    }
}