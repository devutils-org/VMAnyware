//! Generated seed inspection and guarded regeneration; never guest readiness proof.
use super::{network_parts, uri_encode, CloudInitRequest, Error, Proxmox};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SeedPreflight {
    pub config_digest: String,
    pub user_data_hash: String,
    pub network_data_hash: String,
    pub user_format: SeedUserFormat,
    pub compatible: bool,
    pub guest_ready: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SeedUserFormat {
    Named,
    ImageDefault,
    ScalarUnsupported,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UserData {
    hostname: String,
    manage_etc_hosts: bool,
    fqdn: String,
    #[serde(default, deserialize_with = "present_user")]
    user: Option<User>,
    users: Vec<String>,
    ssh_authorized_keys: Vec<String>,
    chpasswd: PasswordPolicy,
    #[serde(default)]
    package_upgrade: bool,
}
fn present_user<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<User>, D::Error> {
    User::deserialize(d).map(Some)
}
#[derive(Deserialize)]
#[serde(untagged)]
enum User {
    Named(NamedUser),
    Scalar(String),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NamedUser {
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PasswordPolicy {
    expire: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Network {
    version: u8,
    config: Vec<NetworkEntry>,
}
#[derive(Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
enum NetworkEntry {
    #[serde(rename = "physical")]
    Physical {
        name: String,
        mac_address: String,
        subnets: Vec<Subnet>,
    },
    #[serde(rename = "nameserver")]
    Nameserver {
        #[serde(default)]
        address: Vec<String>,
        #[serde(default)]
        search: Vec<String>,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Subnet {
    #[serde(rename = "type")]
    kind: String,
}
fn domain(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b))
}
fn validate(
    user: &str,
    network: &str,
    request: &CloudInitRequest,
) -> Result<SeedUserFormat, Error> {
    if user.len() > 65536 || network.len() > 65536 || !user.starts_with("#cloud-config\n") {
        return Err(Error::InvalidResponse);
    }
    let data: UserData = serde_norway::from_str(user).map_err(|_| Error::InvalidResponse)?;
    if data.hostname != request.name
        || !data.manage_etc_hosts
        || !(data.fqdn == request.name
            || data
                .fqdn
                .strip_prefix(&format!("{}.", request.name))
                .is_some_and(domain))
        || data.users != ["default"]
        || data.chpasswd.expire
        || data.package_upgrade
        || data.ssh_authorized_keys.len() != request.key_count
    {
        return Err(Error::InvalidResponse);
    }
    let mut keys = Vec::new();
    for key in data.ssh_authorized_keys {
        let mut key = ssh_key::PublicKey::from_openssh(&key).map_err(|_| Error::InvalidResponse)?;
        key.set_comment("");
        keys.push(key.to_openssh().map_err(|_| Error::InvalidResponse)?);
    }
    if uri_encode(&keys.join("\n")) != request.encoded_keys {
        return Err(Error::InvalidResponse);
    }
    let format = match data.user {
        Some(User::Named(u)) if !request.image_default_user && u.name == request.user => {
            SeedUserFormat::Named
        }
        Some(User::Scalar(u)) if !request.image_default_user && u == request.user => {
            SeedUserFormat::ScalarUnsupported
        }
        None if request.image_default_user => SeedUserFormat::ImageDefault,
        _ => return Err(Error::InvalidResponse),
    };
    let data: Network = serde_norway::from_str(network).map_err(|_| Error::InvalidResponse)?;
    if data.version != 1 || data.config.is_empty() || data.config.len() > 2 {
        return Err(Error::InvalidResponse);
    }
    let mac = network_parts(&request.network)
        .ok_or(Error::Configuration)?
        .0;
    let mut physical = false;
    let mut dns = false;
    for entry in data.config {
        match entry {
            NetworkEntry::Physical {
                name,
                mac_address,
                subnets,
            } => {
                if physical
                    || name != "eth0"
                    || mac_address.to_ascii_uppercase() != mac
                    || subnets.len() != 1
                    || subnets[0].kind != "dhcp4"
                {
                    return Err(Error::InvalidResponse);
                }
                physical = true;
            }
            NetworkEntry::Nameserver { address, search } => {
                if dns
                    || address.len() > 8
                    || search.len() > 8
                    || address
                        .iter()
                        .any(|v| v.parse::<std::net::IpAddr>().is_err())
                    || search.iter().any(|v| !domain(v))
                {
                    return Err(Error::InvalidResponse);
                }
                dns = true;
            }
        }
    }
    if !physical {
        return Err(Error::InvalidResponse);
    }
    Ok(format)
}
impl Proxmox {
    /// Read only built-in generated previews. Custom overrides are excluded by the
    /// settings observer; dumps do not read custom snippets or the attached ISO.
    pub async fn preflight_generated_seed(
        &self,
        request: &CloudInitRequest,
    ) -> Result<SeedPreflight, Error> {
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            let before = self.observe_cloud_init(request).await?;
            let path = [
                "nodes",
                request.target.node(),
                "qemu",
                &request.target.vmid().to_string(),
                "cloudinit",
                "dump",
            ];
            let user: String = self.get_query(&path, &[("type", "user".into())]).await?;
            let network: String = self.get_query(&path, &[("type", "network".into())]).await?;
            let format = validate(&user, &network, request)?;
            let after = self.observe_cloud_init(request).await?;
            if before.resources.target.config_digest != after.resources.target.config_digest {
                return Err(Error::InvalidResponse);
            }
            Ok(SeedPreflight {
                config_digest: after.resources.target.config_digest,
                user_data_hash: format!("{:x}", Sha256::digest(user.as_bytes())),
                network_data_hash: format!("{:x}", Sha256::digest(network.as_bytes())),
                compatible: matches!(format, SeedUserFormat::Named | SeedUserFormat::ImageDefault),
                user_format: format,
                guest_ready: false,
            })
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
}

impl Proxmox {
    /// Call only after a durable regeneration dispatch marker. Never retry an
    /// ambiguous result: generated previews alone cannot establish regeneration.
    pub async fn dispatch_seed_regeneration(
        &self,
        request: &CloudInitRequest,
        expected: &SeedPreflight,
    ) -> Result<SeedPreflight, Error> {
        tokio::time::timeout(std::time::Duration::from_secs(45), async {
            let before = self.preflight_generated_seed(request).await?;
            if !expected.compatible
                || expected.guest_ready
                || !before.compatible
                || before.config_digest != expected.config_digest
                || before.user_data_hash != expected.user_data_hash
                || before.network_data_hash != expected.network_data_hash
                || before.user_format != expected.user_format
            {
                return Err(Error::InvalidResponse);
            }
            // This endpoint accepts node/vmid only: there is no digest parameter
            // or task identifier. Pre/post checks bound the observation, not an
            // atomic compare-and-swap against concurrent operator edits.
            let mut url = self.endpoint.clone();
            url.path_segments_mut()
                .map_err(|_| Error::Configuration)?
                .extend([
                    "api2",
                    "json",
                    "nodes",
                    request.target.node(),
                    "qemu",
                    &request.target.vmid().to_string(),
                    "cloudinit",
                ]);
            let response = self
                .client
                .put(url)
                .header(
                    super::super::header::AUTHORIZATION,
                    self.authorization.clone(),
                )
                .header(
                    super::super::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body("")
                .send()
                .await
                .map_err(super::super::transport_error)?;
            let data: serde_json::Value = Self::decode_response(response).await?;
            if !data.is_null() {
                return Err(Error::InvalidResponse);
            }
            let after = self.preflight_generated_seed(request).await?;
            if !after.compatible
                || after.user_data_hash != before.user_data_hash
                || after.network_data_hash != before.network_data_hash
                || after.user_format != before.user_format
            {
                return Err(Error::InvalidResponse);
            }
            Ok(after)
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
}
