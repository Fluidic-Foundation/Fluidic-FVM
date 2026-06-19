# Cloud Armor security policy for the Fluidic testnet API gateway.
# This policy is referenced by the GKE BackendConfig attached to the nginx
# gateway Service.

resource "google_compute_security_policy" "fluidic_api" {
  name = "fluidic-testnet-api-policy"

  # Default rule: allow traffic that passes all prior rules.
  rule {
    action   = "allow"
    priority = "2147483647"
    match {
      versioned_expr = "SRC_IPS_V1"
      config {
        src_ip_ranges = ["*"]
      }
    }
    description = "Default allow"
  }

  # Rate-based ban for excessive requests from a single IP.
  rule {
    action      = "rate_based_ban"
    priority    = "1000"
    description = "Ban IPs exceeding 600 req/min"
    match {
      expr {
        expression = "true"
      }
    }
    rate_limit_options {
      rate_limit_threshold {
        count        = 600
        interval_sec = 60
      }
      ban_duration_sec = 300
      conform_action   = "allow"
      exceed_action    = "deny(429)"
      enforce_on_key   = "IP"
    }
  }

  # Block common SQLi/XSS patterns at the edge.
  rule {
    action      = "deny(403)"
    priority    = "1001"
    description = "Block SQL injection patterns"
    match {
      expr {
        expression = "evaluatePreconfiguredExpr('sqli-stable')"
      }
    }
    preview = false
  }

  rule {
    action      = "deny(403)"
    priority    = "1002"
    description = "Block XSS patterns"
    match {
      expr {
        expression = "evaluatePreconfiguredExpr('xss-stable')"
      }
    }
    preview = false
  }
}

output "cloud_armor_policy_name" {
  value = google_compute_security_policy.fluidic_api.name
}
