//! os.click API client for downloading operating system ISO files

use crate::error::UsbCreatorError;
use serde::{Deserialize, Serialize};

/// Operating system category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OsCategory {
    Linux,
    Windows,
}

/// Operating system entry from os.click API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatingSystem {
    pub id: String,
    pub name: String,
    pub version: String,
    pub category: OsCategory,
    pub description: Option<String>,
    pub size_mb: Option<u64>,
    pub checksum_sha256: Option<String>,
}

/// API response containing operating systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsListResponse {
    pub success: bool,
    pub data: Vec<OperatingSystem>,
    pub message: Option<String>,
}

/// os.click API client
pub struct OsClickClient {
    base_url: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl OsClickClient {
    /// Create a new os.click API client
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            base_url: "https://api.os.click".to_string(),
            api_key,
            client: reqwest::Client::new(),
        }
    }

    /// List available operating systems by category
    pub async fn list_os_by_category(&self, category: OsCategory) -> Result<OsListResponse, UsbCreatorError> {
        let url = format!("{}/api/v1/os", self.base_url);

        let mut request = self.client.get(&url);

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| UsbCreatorError::Generic(format!("Failed to send request: {}", e)))?;

        if !response.status().is_success() {
            return Err(UsbCreatorError::Generic(
                format!("API request failed with status: {}", response.status())
            ));
        }

        let os_list: OsListResponse = response
            .json()
            .await
            .map_err(|e| UsbCreatorError::Generic(format!("Failed to parse response: {}", e)))?;

        // Filter by category if the API doesn't support it directly
        let filtered_data = os_list.data
            .into_iter()
            .filter(|os| std::mem::discriminant(&os.category) == std::mem::discriminant(&category))
            .collect();

        Ok(OsListResponse {
            success: os_list.success,
            data: filtered_data,
            message: os_list.message,
        })
    }

    /// Get download information for a specific OS
    pub async fn get_download_info(&self, os_id: &str) -> Result<DownloadInfo, UsbCreatorError> {
        let url = format!("{}/api/v1/os/{}/download", self.base_url, os_id);

        let mut request = self.client.get(&url);

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| UsbCreatorError::Generic(format!("Failed to send request: {}", e)))?;

        if !response.status().is_success() {
            return Err(UsbCreatorError::Generic(
                format!("Download request failed with status: {}", response.status())
            ));
        }

        let download_info: DownloadInfo = response
            .json()
            .await
            .map_err(|e| UsbCreatorError::Generic(format!("Failed to parse download response: {}", e)))?;

        Ok(download_info)
    }
}

/// Download information for an operating system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub os_id: String,
    pub download_url: String,
    pub filename: String,
    pub size_bytes: u64,
    pub checksum_sha256: String,
    pub mirrors: Vec<String>,
}

/// Mock function for POC - returns sample data
pub fn mock_list_os_by_category(category: OsCategory) -> OsListResponse {
    match category {
        OsCategory::Linux => {
            OsListResponse {
                success: true,
                data: vec![
                    OperatingSystem {
                        id: "ubuntu-22.04".to_string(),
                        name: "Ubuntu".to_string(),
                        version: "22.04 LTS".to_string(),
                        category: OsCategory::Linux,
                        description: Some("Popular Linux distribution".to_string()),
                        size_mb: Some(3456),
                        checksum_sha256: Some("abc123def456".to_string()),
                    },
                    OperatingSystem {
                        id: "fedora-38".to_string(),
                        name: "Fedora".to_string(),
                        version: "38".to_string(),
                        category: OsCategory::Linux,
                        description: Some("Community Linux distribution".to_string()),
                        size_mb: Some(2890),
                        checksum_sha256: Some("def456ghi789".to_string()),
                    },
                ],
                message: Some("Successfully retrieved Linux distributions".to_string()),
            }
        },
        OsCategory::Windows => {
            OsListResponse {
                success: true,
                data: vec![
                    OperatingSystem {
                        id: "windows-11".to_string(),
                        name: "Windows".to_string(),
                        version: "11 Pro".to_string(),
                        category: OsCategory::Windows,
                        description: Some("Latest Windows version".to_string()),
                        size_mb: Some(5376),
                        checksum_sha256: Some("ghi789jkl012".to_string()),
                    },
                    OperatingSystem {
                        id: "windows-10".to_string(),
                        name: "Windows".to_string(),
                        version: "10 Pro".to_string(),
                        category: OsCategory::Windows,
                        description: Some("Stable Windows version".to_string()),
                        size_mb: Some(4890),
                        checksum_sha256: Some("jkl012mno345".to_string()),
                    },
                ],
                message: Some("Successfully retrieved Windows versions".to_string()),
            }
        },
    }
}