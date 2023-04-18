use std::cmp;
use std::collections::HashMap;

use std::sync::Mutex;

use crate::gatt_server::Profile;
use crate::utilities::AttributeControl;
use esp_idf_sys::ESP_GATT_DEF_BLE_MTU_SIZE;
use esp_idf_sys::*;
use lazy_static::lazy_static;
use log::debug;

lazy_static! {
    static ref MESSAGE_CACHE: Mutex<HashMap<u16, Vec<u8>>> = Mutex::new(HashMap::new());
}

const RESPONSE_LENGTH: usize = 600;
// TODO: Pull current MTU size from MTU update events
const MAX_CHUNK_SIZE: u16 = ESP_GATT_DEF_BLE_MTU_SIZE;

impl Profile {
    pub(crate) fn on_read(
        &mut self,
        gatts_if: esp_gatt_if_t,
        param: esp_ble_gatts_cb_param_t_gatts_read_evt_param,
    ) {
        for service in &self.services {
            service
                .read()
                .unwrap()
                .characteristics
                .iter()
                .for_each(|characteristic| {
                    if characteristic.read().unwrap().attribute_handle == Some(param.handle) {
                        debug!(
                            "Received read event for characteristic {}.",
                            characteristic.read().unwrap()
                        );

                        // If the characteristic has a read handler, call it.
                        if let AttributeControl::ResponseByApp(callback) =
                            &characteristic.read().unwrap().control
                        {
                            let mut locked_cache = MESSAGE_CACHE.lock().unwrap();

                            let value = get_message(&mut locked_cache, param, callback);

                            // Extend the response to the maximum length.
                            let mut response = [0u8; RESPONSE_LENGTH];
                            let possible_max =
                                cmp::min(value.len(), (param.offset + MAX_CHUNK_SIZE).into());
                            let sub_string = &value[param.offset.into()..possible_max];

                            // Remove from the cache once we don't need fragmenting anymore.
                            if sub_string.len() < MAX_CHUNK_SIZE.into() {
                                println!(
                                    "Removing from cache {:?} {:?}",
                                    param.offset, param.handle
                                );
                                locked_cache.remove(&param.handle);
                            }

                            drop(locked_cache);

                            response[..sub_string.len()].copy_from_slice(sub_string);

                            let mut esp_rsp = esp_gatt_rsp_t {
                                attr_value: esp_gatt_value_t {
                                    auth_req: 0,
                                    handle: param.handle,
                                    len: cmp::min(
                                        value.len() as u16 - param.offset,
                                        MAX_CHUNK_SIZE,
                                    ),
                                    offset: param.offset,
                                    value: response,
                                },
                            };

                            unsafe {
                                esp_nofail!(esp_ble_gatts_send_response(
                                    gatts_if,
                                    param.conn_id,
                                    param.trans_id,
                                    // TODO: Allow different statuses.
                                    esp_gatt_status_t_ESP_GATT_OK,
                                    &mut esp_rsp
                                ));
                            }
                        }
                    } else {
                        characteristic
                            .read()
                            .unwrap()
                            .descriptors
                            .iter()
                            .for_each(|descriptor| {
                                debug!(
                                    "MCC: Checking descriptor {} ({:?}).",
                                    descriptor.read().unwrap(),
                                    descriptor.read().unwrap().attribute_handle
                                );

                                if descriptor.read().unwrap().attribute_handle == Some(param.handle)
                                {
                                    debug!(
                                        "Received read event for descriptor {}.",
                                        descriptor.read().unwrap()
                                    );

                                    if let AttributeControl::ResponseByApp(callback) =
                                        &descriptor.read().unwrap().control
                                    {
                                        let value = callback(param);

                                        // Extend the response to the maximum length.
                                        let mut response = [0u8; 600];
                                        response[..value.len()].copy_from_slice(&value);

                                        let mut esp_rsp = esp_gatt_rsp_t {
                                            attr_value: esp_gatt_value_t {
                                                auth_req: 0,
                                                handle: param.handle,
                                                len: value.len() as u16,
                                                offset: 0,
                                                value: response,
                                            },
                                        };

                                        unsafe {
                                            esp_nofail!(esp_ble_gatts_send_response(
                                                gatts_if,
                                                param.conn_id,
                                                param.trans_id,
                                                esp_gatt_status_t_ESP_GATT_OK,
                                                &mut esp_rsp
                                            ));
                                        }
                                    }
                                }
                            });
                    }
                });
        }
    }
}

fn get_message(
    locked_cache: &mut std::sync::MutexGuard<HashMap<u16, Vec<u8>>>,
    param: esp_ble_gatts_cb_param_t_gatts_read_evt_param,
    callback: &std::sync::Arc<
        dyn Fn(esp_ble_gatts_cb_param_t_gatts_read_evt_param) -> Vec<u8> + Send + Sync,
    >,
) -> Vec<u8> {
    let cached_message = locked_cache.get(&param.handle);

    let value = match cached_message {
        Some(message) if param.offset > 0 => message.clone(),
        _ => callback(param),
    };

    if cached_message.is_none() && value.len() > MAX_CHUNK_SIZE.into() {
        locked_cache.insert(param.handle, value.clone());
    }

    value
}
