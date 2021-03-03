use crate::config::{AudioConfig, SpeechConfig};
use crate::error::{convert_err, Result};
use crate::events::EventFactory;
use crate::ffi::{
    recognizer_create_speech_recognizer_from_config, recognizer_handle_release,
    recognizer_recognizing_set_callback, PRECOGNITION_CALLBACK_FUNC, SPXEVENTHANDLE, SPXHR,
    SPXRECOHANDLE,
};
use crate::{SmartHandle, SPXHANDLE_EMPTY};
use log::*;
use std::ffi::c_void;
use tokio::sync::mpsc::Sender;

#[derive(Debug)]
pub struct SpeechRecognizer {
    handle: SmartHandle<SPXRECOHANDLE>,
    speech_config: SpeechConfig,
    audio_config: AudioConfig,
}

impl SpeechRecognizer {
    pub fn from_config(
        speech_config: SpeechConfig,
        audio_config: AudioConfig,
    ) -> Result<SpeechRecognizer> {
        let mut handle = SPXHANDLE_EMPTY;
        unsafe {
            convert_err(
                recognizer_create_speech_recognizer_from_config(
                    &mut handle,
                    speech_config.handle.get(),
                    audio_config.handle.get(),
                ),
                "SpeechRecognizer.from_config error",
            )?;
        }

        Ok(SpeechRecognizer {
            handle: SmartHandle::create("SpeechRecognizer", handle, recognizer_handle_release),
            speech_config,
            audio_config,
        })
    }

    pub fn set_recognizing<T>(&self, sender: Sender<T>) -> Result<()>
    where
        T: EventFactory,
    {
        unsafe {
            convert_err(
                recognizer_recognizing_set_callback(
                    self.handle.get(),
                    Some(Self::cb_send::<T>),
                    &sender as *const _ as *mut c_void,
                ),
                "SpeechRecognizer.set_recognizing failed",
            )?;
        }

        Ok(())
    }

    unsafe extern "C" fn cb_send<T: EventFactory>(
        _hreco: SPXRECOHANDLE,
        h_evt: SPXEVENTHANDLE,
        p_sender: *mut ::std::os::raw::c_void,
    ) {
        let sender = &mut *(p_sender as *mut Sender<T>);
        let event = match T::create(h_evt) {
            Ok(x) => x,
            Err(e) => {
                error!("can not create event, err: {:?}", e);
                return;
            }
        };
        match sender.try_send(event) {
            Ok(()) => {}
            Err(e) => {
                error!("can not publish event, err: {}", e);
            }
        }
    }
}
