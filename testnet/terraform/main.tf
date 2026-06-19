# Fluidic Testnet Infrastructure — Google Cloud

terraform {
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
  }
}

variable "project_id" {
  description = "GCP project ID"
  type        = string
  default     = "project-934c3e12-e0e7-4811-810"
}

variable "region" {
  description = "Primary GCP region"
  type        = string
  default     = "us-central1"
}

variable "domain" {
  description = "Base domain for the testnet"
  type        = string
  default     = "testnet.fluidic.foundation"
}

locals {
  api_domain   = "api.${var.domain}"
  site_domain  = var.domain
  app_domain   = "app.${var.domain}"
  node_count   = 3
}

provider "google" {
  project = var.project_id
  region  = var.region
}

# Enable required APIs
resource "google_project_service" "apis" {
  for_each = toset([
    "compute.googleapis.com",
    "container.googleapis.com",
    "dns.googleapis.com",
    "certificatemanager.googleapis.com",
    "cloudbuild.googleapis.com",
    "artifactregistry.googleapis.com",
    "monitoring.googleapis.com",
    "logging.googleapis.com",
    "storage.googleapis.com",
  ])
  service = each.value
}

# VPC
resource "google_compute_network" "vpc" {
  name                    = "fluidic-testnet-vpc"
  auto_create_subnetworks = false
  depends_on              = [google_project_service.apis]
}

resource "google_compute_subnetwork" "subnet" {
  name          = "fluidic-testnet-subnet"
  ip_cidr_range = "10.0.0.0/16"
  region        = var.region
  network       = google_compute_network.vpc.id
}

# Static IP for the API load balancer
resource "google_compute_global_address" "api_ip" {
  name = "fluidic-testnet-api-ip"
}

resource "google_compute_global_address" "site_ip" {
  name = "fluidic-testnet-site-ip"
}

# Artifact Registry for Docker images
resource "google_artifact_registry_repository" "fluidic" {
  location      = var.region
  repository_id = "fluidic"
  format        = "DOCKER"
  description   = "Fluidic testnet container images"
}

# GKE Autopilot cluster
resource "google_container_cluster" "primary" {
  name     = "fluidic-testnet"
  location = var.region

  network    = google_compute_network.vpc.id
  subnetwork = google_compute_subnetwork.subnet.id

  enable_autopilot = true

  depends_on = [google_project_service.apis]
}

# Cloud DNS managed zone (assumes you own the domain)
resource "google_dns_managed_zone" "fluidic" {
  name        = "fluidic-testnet-zone"
  dns_name    = "${var.domain}."
  description = "Fluidic testnet DNS zone"
}

resource "google_dns_record_set" "api" {
  name         = "${local.api_domain}."
  managed_zone = google_dns_managed_zone.fluidic.name
  type         = "A"
  ttl          = 300
  rrdatas      = [google_compute_global_address.api_ip.address]
}

resource "google_dns_record_set" "site" {
  name         = "${local.site_domain}."
  managed_zone = google_dns_managed_zone.fluidic.name
  type         = "A"
  ttl          = 300
  rrdatas      = [google_compute_global_address.site_ip.address]
}

resource "google_dns_record_set" "app" {
  name         = "${local.app_domain}."
  managed_zone = google_dns_managed_zone.fluidic.name
  type         = "A"
  ttl          = 300
  rrdatas      = [google_compute_global_address.site_ip.address]
}

# Static site buckets
resource "google_storage_bucket" "site" {
  name          = "fluidic-testnet-site"
  location      = var.region
  force_destroy = true

  website {
    main_page_suffix = "index.html"
    not_found_page   = "index.html"
  }

  uniform_bucket_level_access = true
}

resource "google_storage_bucket" "dapp" {
  name          = "fluidic-testnet-dapp"
  location      = var.region
  force_destroy = true

  website {
    main_page_suffix = "index.html"
    not_found_page   = "index.html"
  }

  uniform_bucket_level_access = true
}

resource "google_storage_bucket_iam_member" "site_public" {
  bucket = google_storage_bucket.site.name
  role   = "roles/storage.objectViewer"
  member = "allUsers"
}

resource "google_storage_bucket_iam_member" "dapp_public" {
  bucket = google_storage_bucket.dapp.name
  role   = "roles/storage.objectViewer"
  member = "allUsers"
}

resource "google_compute_backend_bucket" "site" {
  name        = "fluidic-testnet-site-backend"
  bucket_name = google_storage_bucket.site.name
  enable_cdn  = true
}

resource "google_compute_backend_bucket" "dapp" {
  name        = "fluidic-testnet-dapp-backend"
  bucket_name = google_storage_bucket.dapp.name
  enable_cdn  = true
}

resource "google_compute_url_map" "site" {
  name = "fluidic-testnet-site-map"

  default_service = google_compute_backend_bucket.site.id

  host_rule {
    hosts        = [local.site_domain]
    path_matcher = "site"
  }

  host_rule {
    hosts        = [local.app_domain]
    path_matcher = "dapp"
  }

  path_matcher {
    name            = "site"
    default_service = google_compute_backend_bucket.site.id
  }

  path_matcher {
    name            = "dapp"
    default_service = google_compute_backend_bucket.dapp.id
  }
}

resource "google_compute_target_http_proxy" "site" {
  name    = "fluidic-testnet-site-proxy"
  url_map = google_compute_url_map.site.id
}

resource "google_compute_global_forwarding_rule" "site_http" {
  name       = "fluidic-testnet-site-http"
  target     = google_compute_target_http_proxy.site.id
  port_range = "80"
  ip_address = google_compute_global_address.site_ip.address
}

# Outputs
output "api_ip" {
  value = google_compute_global_address.api_ip.address
}

output "site_ip" {
  value = google_compute_global_address.site_ip.address
}

output "cluster_name" {
  value = google_container_cluster.primary.name
}

output "artifact_registry" {
  value = "${var.region}-docker.pkg.dev/${var.project_id}/${google_artifact_registry_repository.fluidic.repository_id}"
}

output "site_bucket" {
  value = google_storage_bucket.site.name
}

output "dapp_bucket" {
  value = google_storage_bucket.dapp.name
}
