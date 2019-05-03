//! Functions for sending actual AT commands to a connected modem.
//!
//! This module's submodules contain various utility functions that will send an AT command to do
//! something with the modem, and will return some form of typed result. Basically, you probably
//! want to take a look in here if you want to do anything useful without having to have a copy of
//! the modem manual yourself!
pub mod network;
pub mod sms;
