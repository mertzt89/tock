IEEE 802.15.4 Stack
===================



Implementation
-------------

Stack overview:

```text
┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ Syscall Interface
┌──────────────────────┐
│     RadioDriver      │
└──────────────────────┘
┄┄ ieee802154::device::MacDevice ┄┄
┌──────────────────────┐
│      VirtualMac      │
└──────────────────────┘
┄┄ ieee802154::device::MacDevice ┄┄
┌──────────────────────┐
│        Framer        │
└──────────────────────┘
┄┄ ieee802154::mac::Mac ┄┄
┌──────────────────────┐
│  MAC (ex: AwakeMac)  │
└──────────────────────┘
┄┄ hil::radio::Radio ┄┄
┌──────────────────────┐
│    802.15.4 Radio    │
└──────────────────────┘
```




mac device

```rust
pub trait MacDevice<'a> {
    fn get_address(&self) -> u16;
    fn get_address_long(&self) -> [u8; 8];
    fn get_pan(&self) -> u16;

    fn set_address(&self, addr: u16);
    fn set_address_long(&self, addr: [u8; 8]);
    fn set_pan(&self, id: u16);

    fn config_commit(&self);
    fn is_on(&self) -> bool;
    fn transmit(&self, frame: Frame) -> Result<(), (ErrorCode, &'static mut [u8])>;

    fn prepare_data_frame(&self, buf: &'static mut [u8], dst_pan: PanID, dst_addr: MacAddress, src_pan: PanID, src_addr: MacAddress, security_needed: Option<(SecurityLevel, KeyId)>) -> Result<Frame, &'static mut [u8]>;
    fn buf_to_frame(&self, buf: &'static mut [u8], len: usize) -> Result<Frame, (ErrorCode, &'static mut [u8])>;
}
```


mac:

```rust
pub trait Mac<'a> {
    fn initialize(&self) -> Result<(), ErrorCode>;

    fn get_address(&self) -> u16;
    fn get_address_long(&self) -> [u8; 8];
    fn get_pan(&self) -> u16;

    fn set_address(&self, addr: u16);    /// Sets the long 64-bit address of the radio
    fn set_address_long(&self, addr: [u8; 8]);
    fn set_pan(&self, id: u16);

    fn config_commit(&self);
    fn is_on(&self) -> bool;
    fn transmit(&self, full_mac_frame: &'static mut [u8], frame_len: usize) -> Result<(), (ErrorCode, &'static mut [u8])>;
}
```


hil::radio::Radio


```rust
pub trait RadioConfig<'a> {
    fn initialize(&self) -> Result<(), ErrorCode>;

    fn reset(&self) -> Result<(), ErrorCode>;
    fn start(&self) -> Result<(), ErrorCode>;
    fn stop(&self) -> Result<(), ErrorCode>;
    fn busy(&self) -> bool;

    fn get_address(&self) -> u16; //....... The local 16-bit address
    fn get_address_long(&self) -> [u8; 8]; // 64-bit address
    fn get_pan(&self) -> u16; //........... The 16-bit PAN ID

    fn get_tx_power(&self) -> i8; //....... The transmit power, in dBm
    fn get_channel(&self) -> u8; // ....... The 802.15.4 channel

    fn set_address(&self, addr: u16);
    fn set_address_long(&self, addr: [u8; 8]);
    fn set_pan(&self, id: u16);

    fn set_tx_power(&self, power: i8) -> Result<(), ErrorCode>;
    fn set_channel(&self, chan: RadioChannel);

    fn config_commit(&self);
    fn is_on(&self) -> bool;
    fn transmit(&self, spi_buf: &'static mut [u8], frame_len: usize) -> Result<(), (ErrorCode, &'static mut [u8])>;
}
```

