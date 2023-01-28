use quick_xml::events;
use quick_xml::events::attributes::AttrError;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::BytesText;
use quick_xml::events::Event;
use quick_xml::Result as XmlResult;
use quick_xml::{Reader, Writer};

use std::borrow::Cow;
use std::io::{BufReader, BufWriter};
use std::net::{Shutdown, TcpStream};

use std::num;
use std::str;

use chrono::format::ParseError;
use chrono::prelude::*;
use std::io::Write;
use std::str::FromStr;

use std::collections::HashMap;
use std::net::ToSocketAddrs;

pub static INDI_PROTOCOL_VERSION: &str = "1.7";

pub mod serialization;
pub use serialization::*;
#[derive(Debug, PartialEq)]
pub enum PropertyState {
    Idle,
    Ok,
    Busy,
    Alert,
}

#[derive(Debug, PartialEq)]
pub enum SwitchState {
    On,
    Off,
}

#[derive(Debug, PartialEq)]
pub enum SwitchRule {
    OneOfMany,
    AtMostOne,
    AnyOfMany,
}

#[derive(Debug, PartialEq)]
pub enum PropertyPerm {
    RO,
    WO,
    RW,
}

#[derive(Debug, PartialEq)]
pub enum BlobEnable {
    Never,
    Also,
    Only,
}

#[derive(Debug, PartialEq)]
pub struct Switch {
    pub label: Option<String>,
    pub value: SwitchState,
}

#[derive(Debug, PartialEq)]
pub struct SwitchVector {
    pub name: String,
    pub group: Option<String>,
    pub label: Option<String>,
    pub state: PropertyState,
    pub perm: PropertyPerm,
    pub rule: SwitchRule,
    pub timeout: Option<u32>,
    pub timestamp: Option<DateTime<Utc>>,

    pub values: HashMap<String, Switch>,
}

#[derive(Debug, PartialEq)]
pub struct Number {
    pub label: Option<String>,
    pub format: String,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub value: f64,
}

#[derive(Debug, PartialEq)]
pub struct NumberVector {
    pub name: String,
    pub group: Option<String>,
    pub label: Option<String>,
    pub state: PropertyState,
    pub perm: PropertyPerm,
    pub timeout: Option<u32>,
    pub timestamp: Option<DateTime<Utc>>,

    pub values: HashMap<String, Number>,
}

#[derive(Debug, PartialEq)]
pub struct Light {
    label: Option<String>,
    value: PropertyState,
}

#[derive(Debug, PartialEq)]
pub struct LightVector {
    pub name: String,
    pub label: Option<String>,
    pub group: Option<String>,
    pub state: PropertyState,
    pub timestamp: Option<DateTime<Utc>>,

    pub values: HashMap<String, Light>,
}

#[derive(Debug, PartialEq)]
pub struct Text {
    pub label: Option<String>,
    pub value: String,
}

#[derive(Debug, PartialEq)]
pub struct TextVector {
    pub name: String,
    pub group: Option<String>,
    pub label: Option<String>,

    pub state: PropertyState,
    pub perm: PropertyPerm,
    pub timeout: Option<u32>,
    pub timestamp: Option<DateTime<Utc>>,

    pub values: HashMap<String, Text>,
}

#[derive(Debug, PartialEq)]
pub struct Blob {
    pub label: Option<String>,
    pub format: Option<String>,
    pub value: Option<Vec<u8>>,
}

#[derive(Debug, PartialEq)]
pub struct BlobVector {
    pub name: String,
    pub label: Option<String>,
    pub group: Option<String>,
    pub state: PropertyState,
    pub perm: PropertyPerm,
    pub timeout: Option<u32>,
    pub timestamp: Option<DateTime<Utc>>,
    pub enable_status: BlobEnable,

    pub values: HashMap<String, Blob>,
}

#[derive(Debug, PartialEq)]
pub enum Parameter {
    TextVector(TextVector),
    NumberVector(NumberVector),
    SwitchVector(SwitchVector),
    LightVector(LightVector),
    BlobVector(BlobVector),
}

impl Parameter {
    pub fn get_group(&self) -> &Option<String> {
        match self {
            Parameter::TextVector(p) => &p.group,
            Parameter::NumberVector(p) => &p.group,
            Parameter::SwitchVector(p) => &p.group,
            Parameter::LightVector(p) => &p.group,
            Parameter::BlobVector(p) => &p.group,
        }
    }

    pub fn get_name(&self) -> &String {
        match self {
            Parameter::TextVector(p) => &p.name,
            Parameter::NumberVector(p) => &p.name,
            Parameter::SwitchVector(p) => &p.name,
            Parameter::LightVector(p) => &p.name,
            Parameter::BlobVector(p) => &p.name,
        }
    }
    pub fn get_label(&self) -> &Option<String> {
        match self {
            Parameter::TextVector(p) => &p.label,
            Parameter::NumberVector(p) => &p.label,
            Parameter::SwitchVector(p) => &p.label,
            Parameter::LightVector(p) => &p.label,
            Parameter::BlobVector(p) => &p.label,
        }
    }
    pub fn get_state(&self) -> &PropertyState {
        match self {
            Parameter::TextVector(p) => &p.state,
            Parameter::NumberVector(p) => &p.state,
            Parameter::SwitchVector(p) => &p.state,
            Parameter::LightVector(p) => &p.state,
            Parameter::BlobVector(p) => &p.state,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum UpdateError {
    ParameterMissing(String),
    ParameterTypeMismatch(String),
}

pub enum Action {
    Define,
    Update,
    Delete,
}

#[derive(Debug)]
pub struct Device {
    parameters: HashMap<String, Parameter>,
    names: Vec<String>,
    groups: Vec<Option<String>>,
}

impl Device {
    pub fn new() -> Device {
        Device {
            parameters: HashMap::new(),
            names: vec![],
            groups: vec![],
        }
    }

    pub fn update(
        &mut self,
        command: serialization::Command,
    ) -> Result<Option<&Parameter>, UpdateError> {
        match command {
            Command::Message(_) => Ok(None),
            Command::GetProperties(_) => Ok(None),
            Command::DefSwitchVector(command) => self.new_param(command),
            Command::SetSwitchVector(command) => self.update_param(command),
            Command::NewSwitchVector(_) => Ok(None),
            Command::DefNumberVector(command) => self.new_param(command),
            Command::SetNumberVector(command) => self.update_param(command),
            Command::NewNumberVector(_) => Ok(None),
            Command::DefTextVector(command) => self.new_param(command),
            Command::SetTextVector(command) => self.update_param(command),
            Command::NewTextVector(_) => Ok(None),
            Command::DefBlobVector(command) => self.new_param(command),
            Command::SetBlobVector(command) => self.update_param(command),
            Command::DefLightVector(command) => self.new_param(command),
            Command::SetLightVector(command) => self.update_param(command),
            Command::DelProperty(command) => self.delete_param(command.name),
            Command::EnableBlob(_) => Ok(None),
        }
    }

    pub fn parameter_names(&self) -> &Vec<String> {
        return &self.names;
    }

    pub fn parameter_groups(&self) -> &Vec<Option<String>> {
        return &self.groups;
    }

    pub fn get_parameters(&self) -> &HashMap<String, Parameter> {
        return &self.parameters;
    }

    fn new_param<T: CommandtoParam>(&mut self, def: T) -> Result<Option<&Parameter>, UpdateError> {
        let name = def.get_name().clone();

        self.names.push(name.clone());
        if let None = self.groups.iter().find(|&x| x == def.get_group()) {
            self.groups.push(def.get_group().clone());
        }

        let param = def.to_param();
        self.parameters.insert(name.clone(), param);
        Ok(self.parameters.get(&name))
    }

    fn update_param<T: CommandToUpdate>(
        &mut self,
        new_command: T,
    ) -> Result<Option<&Parameter>, UpdateError> {
        match self.parameters.get_mut(&new_command.get_name().clone()) {
            Some(param) => {
                new_command.update_param(param)?;
                Ok(Some(param))
            }
            None => Err(UpdateError::ParameterMissing(
                new_command.get_name().clone(),
            )),
        }
    }

    fn delete_param(&mut self, name: Option<String>) -> Result<Option<&Parameter>, UpdateError> {
        match name {
            Some(name) => {
                self.names.retain(|n| *n != name);
                self.parameters.remove(&name);
            }
            None => {
                self.names.clear();
                self.parameters.drain();
            }
        };
        Ok(None)
    }
}

pub trait CommandtoParam {
    fn get_name(&self) -> &String;
    fn get_group(&self) -> &Option<String>;
    fn to_param(self) -> Parameter;
}

pub trait CommandToUpdate {
    fn get_name(&self) -> &String;
    fn update_param(self, param: &mut Parameter) -> Result<String, UpdateError>;
}

/// Struct used to keep track of a the devices and their properties.
/// When used in conjunction with the Connection struct can be used to
/// track and control devices managed by an INDI server.
#[derive(Debug)]
pub struct Client {
    devices: HashMap<String, Device>,
}

impl Client {
    /// Create a new client object.
    pub fn new() -> Client {
        Client {
            devices: HashMap::new(),
        }
    }

    /// Update the state of the appropriate device property for a command that came from an INDI server.
    pub fn update(
        &mut self,
        command: serialization::Command,
    ) -> Result<Option<&Parameter>, UpdateError> {
        let name = command.device_name();
        match name {
            Some(name) => {
                let device = self.devices.entry(name.clone()).or_insert(Device::new());
                device.update(command)
            }
            None => Ok(None),
        }
    }

    /// Accessor for stored devices.
    pub fn get_devices(&self) -> &HashMap<String, Device> {
        return &self.devices;
    }

    /// Clear (aka, empty) the stored devices.
    pub fn clear(&mut self) {
        self.devices.clear();
    }
}

pub struct Connection {
    connection: TcpStream,
    xml_writer: Writer<BufWriter<TcpStream>>,
}

impl Connection {
    /// Creates a new connection to an INDI server at the specified address.
    pub fn new<A: ToSocketAddrs>(addr: A) -> std::io::Result<Connection> {
        let connection = TcpStream::connect(addr)?;
        let xml_writer = Writer::new_with_indent(BufWriter::new(connection.try_clone()?), b' ', 2);

        Ok(Connection {
            connection,
            xml_writer,
        })
    }

    /// Disconnects from the INDI server
    pub fn disconnect(&self) -> Result<(), std::io::Error> {
        self.connection.shutdown(Shutdown::Both)
    }

    /// Creates an interator that yields commands from the the connected INDI server.
    /// Example usage:
    /// ```no_run
    /// let mut connection = indi::Connection::new("localhost:7624").unwrap();
    /// connection.write(&indi::GetProperties {
    ///     version: indi::INDI_PROTOCOL_VERSION.to_string(),
    ///     device: None,
    ///     name: None,
    /// }).unwrap();
    ///
    /// let mut client = indi::Client::new();
    ///
    /// for command in connection.iter().unwrap() {
    ///     println!("Command: {:?}", command);
    ///     client.update(command.unwrap());
    /// }
    pub fn iter(&self) -> Result<serialization::CommandIter<BufReader<TcpStream>>, std::io::Error> {
        let mut xml_reader = Reader::from_reader(BufReader::new(self.connection.try_clone()?));
        xml_reader.trim_text(true);
        xml_reader.expand_empty_elements(true);
        Ok(serialization::CommandIter::new(xml_reader))
    }

    /// Sends the given INDI command to the connected server.  Consumes the command.
    /// Example usage:
    /// ```no_run
    /// let mut connection = indi::Connection::new("localhost:7624").unwrap();
    /// connection.write(&indi::GetProperties {
    ///     version: indi::INDI_PROTOCOL_VERSION.to_string(),
    ///     device: None,
    ///     name: None,
    /// }).unwrap();
    ///
    pub fn write<T: XmlSerialization>(&mut self, command: &T) -> Result<(), DeError> {
        command.write(&mut self.xml_writer)?;
        self.xml_writer.inner().flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod device_tests {
    use super::*;

    #[test]
    fn test_update_switch() {
        let mut device = Device::new();
        let timestamp = DateTime::from_str("2022-10-13T07:41:56.301Z").unwrap();

        let def_switch = DefSwitchVector {
            device: String::from_str("CCD Simulator").unwrap(),
            name: String::from_str("Exposure").unwrap(),
            label: Some(String::from_str("thingo").unwrap()),
            group: Some(String::from_str("group").unwrap()),
            state: PropertyState::Ok,
            perm: PropertyPerm::RW,
            rule: SwitchRule::AtMostOne,
            timeout: Some(60),
            timestamp: Some(timestamp),
            message: None,
            switches: vec![DefSwitch {
                name: String::from_str("seconds").unwrap(),
                label: Some(String::from_str("asdf").unwrap()),
                value: SwitchState::On,
            }],
        };
        assert_eq!(device.get_parameters().len(), 0);
        device
            .update(serialization::Command::DefSwitchVector(def_switch))
            .unwrap();
        assert_eq!(device.get_parameters().len(), 1);

        if let Parameter::SwitchVector(stored) = device.get_parameters().get("Exposure").unwrap() {
            assert_eq!(
                stored,
                &SwitchVector {
                    name: String::from_str("Exposure").unwrap(),
                    group: Some(String::from_str("group").unwrap()),
                    label: Some(String::from_str("thingo").unwrap()),
                    state: PropertyState::Ok,
                    perm: PropertyPerm::RW,
                    rule: SwitchRule::AtMostOne,
                    timeout: Some(60),
                    timestamp: Some(timestamp),
                    values: HashMap::from([(
                        String::from_str("seconds").unwrap(),
                        Switch {
                            label: Some(String::from_str("asdf").unwrap()),
                            value: SwitchState::On
                        }
                    )])
                }
            );
        } else {
            panic!("Unexpected");
        }

        let timestamp = DateTime::from_str("2022-10-13T08:41:56.301Z").unwrap();
        let set_switch = SetSwitchVector {
            device: String::from_str("CCD Simulator").unwrap(),
            name: String::from_str("Exposure").unwrap(),
            state: PropertyState::Ok,
            timeout: Some(60),
            timestamp: Some(timestamp),
            message: None,
            switches: vec![OneSwitch {
                name: String::from_str("seconds").unwrap(),
                value: SwitchState::Off,
            }],
        };
        assert_eq!(device.get_parameters().len(), 1);
        device
            .update(serialization::Command::SetSwitchVector(set_switch))
            .unwrap();
        assert_eq!(device.get_parameters().len(), 1);

        if let Parameter::SwitchVector(stored) = device.get_parameters().get("Exposure").unwrap() {
            assert_eq!(
                stored,
                &SwitchVector {
                    name: String::from_str("Exposure").unwrap(),
                    group: Some(String::from_str("group").unwrap()),
                    label: Some(String::from_str("thingo").unwrap()),
                    state: PropertyState::Ok,
                    perm: PropertyPerm::RW,
                    rule: SwitchRule::AtMostOne,
                    timeout: Some(60),
                    timestamp: Some(timestamp),
                    values: HashMap::from([(
                        String::from_str("seconds").unwrap(),
                        Switch {
                            label: Some(String::from_str("asdf").unwrap()),
                            value: SwitchState::Off
                        }
                    )])
                }
            );
        } else {
            panic!("Unexpected");
        }
    }

    #[test]
    fn test_update_number() {
        let mut device = Device::new();
        let timestamp = DateTime::from_str("2022-10-13T07:41:56.301Z").unwrap();

        let def_number = DefNumberVector {
            device: String::from_str("CCD Simulator").unwrap(),
            name: String::from_str("Exposure").unwrap(),
            label: Some(String::from_str("thingo").unwrap()),
            group: Some(String::from_str("group").unwrap()),
            state: PropertyState::Ok,
            perm: PropertyPerm::RW,
            timeout: Some(60),
            timestamp: Some(timestamp),
            message: None,
            numbers: vec![DefNumber {
                name: String::from_str("seconds").unwrap(),
                label: Some(String::from_str("asdf").unwrap()),
                format: String::from_str("%4.0f").unwrap(),
                min: 0.0,
                max: 100.0,
                step: 1.0,
                value: 13.3,
            }],
        };
        assert_eq!(device.get_parameters().len(), 0);
        device
            .update(serialization::Command::DefNumberVector(def_number))
            .unwrap();
        assert_eq!(device.get_parameters().len(), 1);

        if let Parameter::NumberVector(stored) = device.get_parameters().get("Exposure").unwrap() {
            assert_eq!(
                stored,
                &NumberVector {
                    name: String::from_str("Exposure").unwrap(),
                    group: Some(String::from_str("group").unwrap()),
                    label: Some(String::from_str("thingo").unwrap()),
                    state: PropertyState::Ok,
                    perm: PropertyPerm::RW,
                    timeout: Some(60),
                    timestamp: Some(timestamp),
                    values: HashMap::from([(
                        String::from_str("seconds").unwrap(),
                        Number {
                            label: Some(String::from_str("asdf").unwrap()),
                            format: String::from_str("%4.0f").unwrap(),
                            min: 0.0,
                            max: 100.0,
                            step: 1.0,
                            value: 13.3,
                        }
                    )])
                }
            );
        } else {
            panic!("Unexpected");
        }

        let timestamp = DateTime::from_str("2022-10-13T08:41:56.301Z").unwrap();
        let set_number = SetNumberVector {
            device: String::from_str("CCD Simulator").unwrap(),
            name: String::from_str("Exposure").unwrap(),
            state: PropertyState::Ok,
            timeout: Some(60),
            timestamp: Some(timestamp),
            message: None,
            numbers: vec![OneNumber {
                name: String::from_str("seconds").unwrap(),
                min: None,
                max: None,
                step: None,
                value: 5.0,
            }],
        };
        assert_eq!(device.get_parameters().len(), 1);
        device
            .update(serialization::Command::SetNumberVector(set_number))
            .unwrap();
        assert_eq!(device.get_parameters().len(), 1);

        if let Parameter::NumberVector(stored) = device.get_parameters().get("Exposure").unwrap() {
            assert_eq!(
                stored,
                &NumberVector {
                    name: String::from_str("Exposure").unwrap(),
                    group: Some(String::from_str("group").unwrap()),
                    label: Some(String::from_str("thingo").unwrap()),
                    state: PropertyState::Ok,
                    perm: PropertyPerm::RW,
                    timeout: Some(60),
                    timestamp: Some(timestamp),
                    values: HashMap::from([(
                        String::from_str("seconds").unwrap(),
                        Number {
                            label: Some(String::from_str("asdf").unwrap()),
                            format: String::from_str("%4.0f").unwrap(),
                            min: 0.0,
                            max: 100.0,
                            step: 1.0,
                            value: 5.0
                        }
                    )])
                }
            );
        } else {
            panic!("Unexpected");
        }
    }

    #[test]
    fn test_update_text() {
        let mut device = Device::new();
        let timestamp = DateTime::from_str("2022-10-13T07:41:56.301Z").unwrap();

        let def_text = DefTextVector {
            device: String::from_str("CCD Simulator").unwrap(),
            name: String::from_str("Exposure").unwrap(),
            label: Some(String::from_str("thingo").unwrap()),
            group: Some(String::from_str("group").unwrap()),
            state: PropertyState::Ok,
            perm: PropertyPerm::RW,
            timeout: Some(60),
            timestamp: Some(timestamp),
            message: None,
            texts: vec![DefText {
                name: String::from_str("seconds").unwrap(),
                label: Some(String::from_str("asdf").unwrap()),
                value: String::from_str("something").unwrap(),
            }],
        };
        assert_eq!(device.get_parameters().len(), 0);
        device
            .update(serialization::Command::DefTextVector(def_text))
            .unwrap();
        assert_eq!(device.get_parameters().len(), 1);

        if let Parameter::TextVector(stored) = device.get_parameters().get("Exposure").unwrap() {
            assert_eq!(
                stored,
                &TextVector {
                    name: String::from_str("Exposure").unwrap(),
                    group: Some(String::from_str("group").unwrap()),
                    label: Some(String::from_str("thingo").unwrap()),
                    state: PropertyState::Ok,
                    perm: PropertyPerm::RW,
                    timeout: Some(60),
                    timestamp: Some(timestamp),
                    values: HashMap::from([(
                        String::from_str("seconds").unwrap(),
                        Text {
                            label: Some(String::from_str("asdf").unwrap()),
                            value: String::from_str("something").unwrap(),
                        }
                    )])
                }
            );
        } else {
            panic!("Unexpected");
        }

        let timestamp = DateTime::from_str("2022-10-13T08:41:56.301Z").unwrap();
        let set_number = SetTextVector {
            device: String::from_str("CCD Simulator").unwrap(),
            name: String::from_str("Exposure").unwrap(),
            state: PropertyState::Ok,
            timeout: Some(60),
            timestamp: Some(timestamp),
            message: None,
            texts: vec![OneText {
                name: String::from_str("seconds").unwrap(),
                value: String::from_str("something else").unwrap(),
            }],
        };
        assert_eq!(device.get_parameters().len(), 1);
        device
            .update(serialization::Command::SetTextVector(set_number))
            .unwrap();
        assert_eq!(device.get_parameters().len(), 1);

        if let Parameter::TextVector(stored) = device.get_parameters().get("Exposure").unwrap() {
            assert_eq!(
                stored,
                &TextVector {
                    name: String::from_str("Exposure").unwrap(),
                    group: Some(String::from_str("group").unwrap()),
                    label: Some(String::from_str("thingo").unwrap()),
                    state: PropertyState::Ok,
                    perm: PropertyPerm::RW,
                    timeout: Some(60),
                    timestamp: Some(timestamp),
                    values: HashMap::from([(
                        String::from_str("seconds").unwrap(),
                        Text {
                            label: Some(String::from_str("asdf").unwrap()),
                            value: String::from_str("something else").unwrap(),
                        }
                    )])
                }
            );
        } else {
            panic!("Unexpected");
        }
    }
}
