use windows::{core::{GUID, VARIANT}, Win32::System::Com::{IDispatch, ITypeInfo, DISPATCH_FLAGS, DISPPARAMS, EXCEPINFO, TYPEATTR}};

use crate::{wide, WinError, LOCALE_USER_DEFAULT};

use super::variant::{EvilVariant, TypedVariant};

#[derive(Debug)]
pub enum DispatchError {
    InvokeError {
        invoked_name : String,
        error : windows::core::Error,
        exception : EXCEPINFO,
    },
    DispidError {
        name : String,
        error : windows::core::Error,
    }
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvokeError { invoked_name, error, exception } => 
            write!(f, 
                "Invoked name : {}, error : {}, EXCEPINFO : {:?}", invoked_name, error, exception),
            Self::DispidError { name, error } => 
            write!(f,
                "Name : {}, error : {}", name, error)
        }
    }
}


#[repr(u16)]
pub(crate) enum Invocation {
    Method = 1,
    PropertyGet = 2,
    PropertySet = 4,
    MethodByref = 1 | 8,
    PropertyGetByRef = 2 | 8,
    PropertySetByRef = 4 | 8,
    Byref = 8,
}


pub(crate) trait HasDispatch {
    fn dispatch(&self) -> &IDispatch;

    fn get_dispid(&self, member_name : &str) -> Result<i32, WinError> {
        let mut rgdispid : i32 = 0;
        let wide_str = wide(member_name);
        
        if let Err(e) = unsafe {
            self.dispatch().GetIDsOfNames(
                &GUID::zeroed(), // Useless param
                &wide_str, // Method name
                1, // # of method names
                LOCALE_USER_DEFAULT, // Localization
                &mut rgdispid // dispid pointer
            )
        } {
            return Err(WinError::DispatchError(DispatchError::DispidError {
                name : member_name.to_string(),
                error : e
            }))
        };

        Ok(rgdispid)
    }

    fn prop(&self, property_name : &str) -> Result<TypedVariant, WinError> {
        self.call(property_name, Invocation::PropertyGet, None)
    }
     
    fn call_raw(&self, method_name : &str, flag : Invocation, args : Option<DISPPARAMS>) -> Result<VARIANT, WinError> {
        let dispatch = self.dispatch();

        let dispid = self.get_dispid(method_name)?;

        let mut exception : EXCEPINFO = EXCEPINFO::default();

        let mut result = VARIANT::default();
        unsafe {
            if let Err(error) = dispatch.Invoke(
                dispid,
                &GUID::zeroed(),
                LOCALE_USER_DEFAULT,
                DISPATCH_FLAGS(flag as u16),
                &args.unwrap_or_default(),
                Some(&mut result),
                Some(&mut exception),
                None,
            ) {
                return Err(WinError::DispatchError(DispatchError::InvokeError { exception, error, invoked_name: method_name.to_string() }))
            };
        };

        Ok(result)
    }

    fn call_evil(&self, method_name : &str, flag : Invocation, args : Option<DISPPARAMS>) -> Result<EvilVariant, WinError> {
        let native = self.call_raw(method_name, flag, args)?;
        Ok(EvilVariant::from(native))
    }

    fn call(&self, method_name : &str, flag : Invocation, args : Option<DISPPARAMS>) -> Result<TypedVariant, WinError> {
        let native = self.call_raw(method_name, flag, args)?;

        Ok(
            TypedVariant::try_from(native)?
        )
    }

    fn get_guid(&self) -> GUID {
        let type_info : ITypeInfo = unsafe{ self.dispatch().GetTypeInfo(0, LOCALE_USER_DEFAULT) }.unwrap();
    
        let attr_ptr = unsafe { type_info.GetTypeAttr()}.unwrap();
        if attr_ptr.is_null() {
            panic!("attribute is null")
        }

        let attr : TYPEATTR = unsafe {*attr_ptr };
        attr.guid
    }
    
    fn dispparams(mut vars : Vec<VARIANT>) -> DISPPARAMS {
        DISPPARAMS {
            rgvarg : vars.as_mut_ptr(),
            cArgs : vars.len() as u32,
            ..Default::default()
        }
    }
}


impl HasDispatch for IDispatch {
    fn dispatch(&self) -> &IDispatch {
        self
    }
}
