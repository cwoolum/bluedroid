use crate::gatt_server::GattServer;
use log::debug;

impl GattServer {
    #[allow(clippy::unused_self)]
    pub(crate) fn on_mtu_change(
        &self,
        param: esp_idf_sys::esp_ble_gatts_cb_param_t_gatts_mtu_evt_param,
    ) {
        let connection = self.active_connections.get(&param.conn_id).unwrap();
        connection.mtu = param.mtu.into();

        debug!("MTU changed to {}.", param.mtu);
    }
}
