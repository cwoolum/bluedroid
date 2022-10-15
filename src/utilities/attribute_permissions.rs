use esp_idf_sys::*;

#[derive(Debug, Clone, Copy)]
pub struct AttributePermissions {
    pub read_access: bool,
    pub write_access: bool,
    pub encryption_required: bool,
}

impl Default for AttributePermissions {
    fn default() -> Self {
        Self {
            read_access: false,
            write_access: false,
            encryption_required: false,
        }
    }
}

impl From<AttributePermissions> for esp_gatt_perm_t {
    fn from(permissions: AttributePermissions) -> Self {
        let result = match (
            permissions.read_access,
            permissions.write_access,
            permissions.encryption_required,
        ) {
            // TODO: Implement all the supported modes.
            (false, false, _) => 0,
            (true, false, false) => ESP_GATT_PERM_READ,
            (false, true, false) => ESP_GATT_PERM_WRITE,
            (true, true, false) => ESP_GATT_PERM_READ | ESP_GATT_PERM_WRITE,
            (true, false, true) => ESP_GATT_PERM_READ_ENCRYPTED,
            (false, true, true) => ESP_GATT_PERM_WRITE_ENCRYPTED,
            (true, true, true) => ESP_GATT_PERM_READ_ENCRYPTED | ESP_GATT_PERM_WRITE_ENCRYPTED,
        };

        result as Self
    }
}
