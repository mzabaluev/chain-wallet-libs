//! This module expose handy C compatible functions to reuse in the different
//! C style bindings that we have (wallet-c, wallet-jni...)

use crate::{Conversion, Error, Result, Wallet};
use chain_impl_mockchain::{transaction::Input, value::Value};
use thiserror::Error;
pub use wallet::Settings;

pub type WalletPtr = *mut Wallet;
pub type SettingsPtr = *mut Settings;
pub type ConversionPtr = *mut Conversion;
pub type ErrorPtr = *mut Error;

#[derive(Debug, Error)]
#[error("null pointer")]
struct NulPtr;

#[derive(Debug, Error)]
#[error("access out of bound")]
struct OutOfBound;

/// retrieve a wallet from the given mnemonics, password and protocol magic
///
/// this function will work for all yoroi, daedalus and other wallets
/// as it will try every kind of wallet anyway
///
/// You can also use this function to recover a wallet even after you have
/// transferred all the funds to the new format (see the _convert_ function)
///
/// The recovered wallet will be returned in `wallet_out`.
///
/// # parameters
///
/// * mnemonics: a null terminated utf8 string (already normalized NFKD) in english;
/// * password: pointer to the password (in bytes, can be UTF8 string or a bytes of anything);
///   this value is optional and passing a null pointer will result in no password;
/// * password_length: the length of the password;
/// * wallet_out: a pointer to a pointer. The recovered wallet will be allocated on this pointer;
///
/// # Safety
///
/// This function dereference raw pointers (password and wallet_out). Even though
/// the function checks if the pointers are null. Mind not to put random values
/// in or you may see unexpected behaviors
///
/// # errors
///
/// The function may fail if:
///
/// * the mnemonics are not valid (invalid length or checksum);
/// * the `wallet_out` is null pointer
///
pub unsafe fn wallet_recover(
    mnemonics: &str,
    password: *const u8,
    password_length: usize,
    wallet_out: *mut WalletPtr,
) -> Result {
    let wallet_out: &mut WalletPtr = if let Some(wallet_out) = wallet_out.as_mut() {
        wallet_out
    } else {
        return Error::invalid_input("wallet_out").with(NulPtr).into();
    };

    let result = if !password.is_null() && password_length > 0 {
        todo!()
    } else {
        Wallet::recover(mnemonics, &[])
    };

    match result {
        Ok(wallet) => {
            *wallet_out = Box::into_raw(Box::new(wallet));
            Result::success()
        }
        Err(err) => err.into(),
    }
}

/// retrieve funds from daedalus or yoroi wallet in the given block0 (or
/// any other blocks).
///
/// Execute this function then you can check who much funds you have
/// retrieved from the given block.
///
/// this function may take sometimes so it is better to only call this
/// function if needed.
///
/// # Safety
///
/// This function dereference raw pointers (wallet, block0 and settings_out). Even though
/// the function checks if the pointers are null. Mind not to put random values
/// in or you may see unexpected behaviors
///
/// # Parameters
///
/// * wallet: the recovered wallet (see recover function);
/// * block0: the pointer to the bytes of the block0;
/// * block0_length: the length of the block0 byte string;
/// * settings_out: the settings that will be parsed from the given
///   block0;
///
/// # Errors
///
/// * this function may fail if the wallet pointer is null;
/// * the block is not valid (cannot be decoded)
///
pub unsafe fn wallet_retrieve_funds(
    wallet: WalletPtr,
    block0: *const u8,
    block0_length: usize,
    settings_out: *mut SettingsPtr,
) -> Result {
    let wallet: &mut Wallet = if let Some(wallet) = wallet.as_mut() {
        wallet
    } else {
        return Error::invalid_input("wallet").with(NulPtr).into();
    };
    if block0.is_null() {
        return Error::invalid_input("block0").with(NulPtr).into();
    }
    let settings_out: &mut SettingsPtr = if let Some(settings_out) = settings_out.as_mut() {
        settings_out
    } else {
        return Error::invalid_input("settings_out").with(NulPtr).into();
    };

    let block0_bytes = std::slice::from_raw_parts(block0, block0_length);

    match wallet.retrieve_funds(block0_bytes) {
        Ok(settings) => {
            *settings_out = Box::into_raw(Box::new(settings));
            Result::success()
        }
        Err(err) => err.into(),
    }
}

/// once funds have been retrieved with `iohk_jormungandr_wallet_retrieve_funds`
/// it is possible to convert all existing funds to the new wallet.
///
/// The returned arrays are transactions to send to the network in order to do the
/// funds conversion.
///
/// Don't forget to call `iohk_jormungandr_wallet_delete_conversion` to
/// properly free the memory
///
/// # Safety
///
/// This function dereference raw pointers (wallet, settings and conversion_out). Even though
/// the function checks if the pointers are null. Mind not to put random values
/// in or you may see unexpected behaviors
///
pub unsafe fn wallet_convert(
    wallet: WalletPtr,
    settings: SettingsPtr,
    conversion_out: *mut ConversionPtr,
) -> Result {
    let wallet: &mut Wallet = if let Some(wallet) = wallet.as_mut() {
        wallet
    } else {
        return Error::invalid_input("wallet").with(NulPtr).into();
    };
    let settings = if let Some(settings) = settings.as_ref() {
        settings.clone()
    } else {
        return Error::invalid_input("settings").with(NulPtr).into();
    };
    let conversion_out: &mut ConversionPtr = if let Some(conversion_out) = conversion_out.as_mut() {
        conversion_out
    } else {
        return Error::invalid_input("conversion_out").with(NulPtr).into();
    };

    let conversion = wallet.convert(settings);

    *conversion_out = Box::into_raw(Box::new(conversion));

    Result::success()
}

/// get the number of transactions built to convert the retrieved wallet
///
/// # Safety
///
/// This function dereference raw pointers. Even though
/// the function checks if the pointers are null. Mind not to put random values
/// in or you may see unexpected behaviors
///
pub unsafe fn wallet_convert_transactions_size(conversion: ConversionPtr) -> usize {
    conversion
        .as_ref()
        .map(|c| c.transactions.len())
        .unwrap_or_default()
}

/// retrieve the index-nth transactions in the conversions starting from 0
/// and finishing at `size-1` where size is retrieved from
/// `iohk_jormungandr_wallet_convert_transactions_size`.
///
/// the memory allocated returned is not owned and should not be kept
/// for longer than potential call to `iohk_jormungandr_wallet_delete_conversion`
///
/// # Safety
///
/// This function dereference raw pointers. Even though
/// the function checks if the pointers are null. Mind not to put random values
/// in or you may see unexpected behaviors
///
pub unsafe fn wallet_convert_transactions_get(
    conversion: ConversionPtr,
    index: usize,
    transaction_out: *mut *const u8,
    transaction_size: *mut usize,
) -> Result {
    let conversion = if let Some(conversion) = conversion.as_ref() {
        conversion
    } else {
        return Error::invalid_input("conversion").with(NulPtr).into();
    };
    let transaction_out = if let Some(t) = transaction_out.as_mut() {
        t
    } else {
        return Error::invalid_input("transaction_out").with(NulPtr).into();
    };
    let transaction_size = if let Some(t) = transaction_size.as_mut() {
        t
    } else {
        return Error::invalid_input("transaction_size").with(NulPtr).into();
    };

    if let Some(t) = conversion.transactions.get(index) {
        *transaction_out = t.as_ref().as_ptr();
        *transaction_size = t.as_ref().len();
        Result::success()
    } else {
        Error::wallet_conversion().with(OutOfBound).into()
    }
}

/// get the total value ignored in the conversion
///
/// value_out: will returns the total value lost into dust inputs
/// ignored_out: will returns the number of dust utxos
///
/// these returned values are informational only and this show that
/// there are UTxOs entries that are unusable because of the way they
/// are populated with dusts.
///
/// # Safety
///
/// This function dereference raw pointers. Even though
/// the function checks if the pointers are null. Mind not to put random values
/// in or you may see unexpected behaviors
///
pub unsafe fn wallet_convert_ignored(
    conversion: ConversionPtr,
    value_out: *mut u64,
    ignored_out: *mut usize,
) -> Result {
    if let Some(c) = conversion.as_ref() {
        let v = *c
            .ignored
            .iter()
            .map(|i: &Input| i.value())
            .sum::<Value>()
            .as_ref();
        let l = c.ignored.len();

        if let Some(value_out) = value_out.as_mut() {
            *value_out = v
        }
        if let Some(ignored_out) = ignored_out.as_mut() {
            *ignored_out = l
        };

        Result::success()
    } else {
        Error::invalid_input("conversion").with(NulPtr).into()
    }
}

/// get the total value in the wallet
///
/// make sure to call `retrieve_funds` prior to calling this function
/// otherwise you will always have `0`
///
/// After calling this function the results is returned in the `total_out`.
///
/// # Errors
///
/// * this function may fail if the wallet pointer is null;
///
/// If the `total_out` pointer is null, this function does nothing
///
/// # Safety
///
/// This function dereference raw pointers. Even though
/// the function checks if the pointers are null. Mind not to put random values
/// in or you may see unexpected behaviors
///
pub unsafe fn wallet_total_value(wallet: WalletPtr, total_out: *mut u64) -> Result {
    let wallet = if let Some(wallet) = wallet.as_ref() {
        wallet
    } else {
        return Error::invalid_input("wallet").with(NulPtr).into();
    };

    if let Some(total_out) = total_out.as_mut() {
        let total = wallet.total_value();

        *total_out = *total.as_ref();
    }

    Result::success()
}

/// update the wallet account state
///
/// this is the value retrieved from any jormungandr endpoint that allows to query
/// for the account state. It gives the value associated to the account as well as
/// the counter.
///
/// It is important to be sure to have an updated wallet state before doing any
/// transactions otherwise future transactions may fail to be accepted by any
/// nodes of the blockchain because of invalid signature state.
///
/// # Errors
///
/// * this function may fail if the wallet pointer is null;
///
pub fn wallet_set_state(wallet: WalletPtr, value: u64, counter: u32) -> Result {
    let wallet = if let Some(wallet) = unsafe { wallet.as_mut() } {
        wallet
    } else {
        return Error::invalid_input("wallet").with(NulPtr).into();
    };
    let value = Value(value);

    wallet.set_state(value, counter);

    Result::success()
}

/// delete the pointer and free the allocated memory
pub fn wallet_delete_error(error: ErrorPtr) {
    if !error.is_null() {
        let boxed = unsafe { Box::from_raw(error) };

        std::mem::drop(boxed);
    }
}

/// delete the pointer and free the allocated memory
pub fn wallet_delete_settings(settings: SettingsPtr) {
    if !settings.is_null() {
        let boxed = unsafe { Box::from_raw(settings) };

        std::mem::drop(boxed);
    }
}

/// delete the pointer, zero all the keys and free the allocated memory
pub fn wallet_delete_wallet(wallet: WalletPtr) {
    if !wallet.is_null() {
        let boxed = unsafe { Box::from_raw(wallet) };

        std::mem::drop(boxed);
    }
}

/// delete the pointer
pub fn wallet_delete_conversion(conversion: ConversionPtr) {
    if !conversion.is_null() {
        let boxed = unsafe { Box::from_raw(conversion) };

        std::mem::drop(boxed);
    }
}