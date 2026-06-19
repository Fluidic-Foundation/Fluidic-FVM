# Basic monitoring for the Fluidic testnet API.
# Creates an uptime check against the public health endpoint and an alerting
# policy that notifies if the endpoint is unavailable for 5 minutes.

resource "google_monitoring_uptime_check_config" "api_health" {
  display_name = "Fluidic testnet API health"
  timeout      = "10s"
  period       = "60s"

  http_check {
    path         = "/api/health"
    port         = 443
    use_ssl      = true
    validate_ssl = true
  }

  monitored_resource {
    type = "uptime_url"
    labels = {
      project_id = var.project_id
      host       = "api.${var.domain}"
    }
  }
}

resource "google_monitoring_alert_policy" "api_down" {
  display_name = "Fluidic API down"
  combiner     = "OR"

  conditions {
    display_name = "API health check failing"

    condition_threshold {
      filter          = "resource.type=\"uptime_url\" AND metric.type=\"monitoring.googleapis.com/uptime_check/check_passed\""
      duration        = "300s"
      comparison      = "COMPARISON_LT"
      threshold_value = 1

      aggregations {
        alignment_period   = "300s"
        per_series_aligner = "ALIGN_FRACTION_TRUE"
      }

      trigger {
        count = 1
      }
    }
  }

  notification_channels = []
  user_labels = {
    service = "fluidic-api"
  }
}
