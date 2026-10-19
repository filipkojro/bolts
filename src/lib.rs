pub mod models;
pub mod reader;
pub mod writer;

use crate::writer::convert_dat_file;

use jni::{
    Env, EnvUnowned,
    errors::{Result, ThrowRuntimeExAndDefault},
    objects::{JClass, JString},
    sys::jint,
};

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_Main_add(_env: &mut Env, _class: JClass, a: jint, b: jint) -> jint {
    return a + b;
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_Main_convertToBolt<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    filename: JString<'caller>,
) {
    unowned_env
        .with_env(|env| -> Result<()> {
            let filename_string = filename.try_to_string(env)?;
            convert_dat_file(filename_string);
            Ok(())
        })
        .resolve::<ThrowRuntimeExAndDefault>();
}
