//! Set slave addresses using `client.init()` and request pre-operational state for both slaves.
//!
//! This is designed for use with the EK1100 + EL2004. An LED should be seen blinking on the second
//! output.

use async_ctrlc::CtrlC;
use ethercrab::al_status::AlState;
use ethercrab::client::Client;
use ethercrab::error::PduError;
use ethercrab::pdu::CheckWorkingCounter;
use ethercrab::register::RegisterAddress;
use ethercrab::std::tx_rx_task;
use ethercrab::sync_manager_channel::{Direction, OperationMode, SyncManagerChannel};
use futures_lite::FutureExt;
use packed_struct::PackedStruct;
use smol::LocalExecutor;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

#[cfg(target_os = "windows")]
// ASRock NIC
// const INTERFACE: &str = "TODO";
// USB NIC
// const INTERFACE: &str = "\\Device\\NPF_{DCEDC919-0A20-47A2-9788-FC57D0169EDB}";
// Silver USB NIC
const INTERFACE: &str = "\\Device\\NPF_{CC0908D5-3CB8-46D6-B8A2-575D0578008D}";
#[cfg(not(target_os = "windows"))]
const INTERFACE: &str = "eth1";

fn main() -> Result<(), PduError> {
    env_logger::init();
    let local_ex = LocalExecutor::new();

    let ctrlc = CtrlC::new().expect("cannot create Ctrl+C handler?");

    futures_lite::future::block_on(local_ex.run(ctrlc.race(async {
        let client = Arc::new(Client::<16, 16, 16, smol::Timer>::new());

        local_ex
            .spawn(tx_rx_task(INTERFACE, &client).unwrap())
            .detach();

        let (_res, num_slaves) = client.brd::<u8>(RegisterAddress::Type).await.unwrap();

        log::info!("Discovered {num_slaves} slaves");

        client.init().await.expect("Init");

        let write_sm = SyncManagerChannel {
            physical_start_address: 0x0000,
            length: 1,
            control: ethercrab::sync_manager_channel::Control {
                operation_mode: OperationMode::Buffered,
                direction: Direction::MasterWrite,
                ..Default::default()
            },
            enable: ethercrab::sync_manager_channel::Enable {
                enable: true,
                ..Default::default()
            },
        };

        client
            .fpwr(0x1001, RegisterAddress::Sm0, write_sm.pack().unwrap())
            .await
            .unwrap()
            .wkc(1, "SM0")
            .unwrap();

        for slave in 0..num_slaves {
            client
                .request_slave_state(usize::from(slave), AlState::PreOp)
                .await
                .expect(&format!("Slave PRE-OP {slave}"));

            client
                .request_slave_state(usize::from(slave), AlState::SafeOp)
                .await
                .expect(&format!("Slave SAFE-OP {slave}"));

            client
                .request_slave_state(usize::from(slave), AlState::Op)
                .await
                .expect(&format!("Slave OP {slave}"));
        }

        let value = Rc::new(RefCell::new(0x00u8));

        let value2 = value.clone();
        let client2 = client.clone();

        // PD TX task (no RX because EL2004 is WO)
        local_ex
            .spawn(async move {
                loop {
                    let v: u8 = *value2.borrow();

                    client2.lwr(0u32, v).await.expect("Bad write");

                    // Cycle time
                    async_io::Timer::after(Duration::from_millis(2)).await;
                }
            })
            .detach();

        loop {
            if *value.borrow() == 0 {
                *value.borrow_mut() = 0b0000_0010;
            } else {
                *value.borrow_mut() = 0;
            }

            async_io::Timer::after(Duration::from_millis(250)).await;
        }
    })));

    Ok(())
}
