// Automatically generated rust module for 'map.proto' file

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(unknown_lints)]
#![allow(clippy::all)]
#![cfg_attr(rustfmt, rustfmt_skip)]


use std::borrow::Cow;
use quick_protobuf::{MessageRead, MessageWrite, BytesReader, Writer, WriterBackend, Result};
use quick_protobuf::sizeofs::*;
use super::*;

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl<'a> MessageRead<'a> for Vector3 {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(13) => msg.x = r.read_float(bytes)?,
                Ok(21) => msg.y = r.read_float(bytes)?,
                Ok(29) => msg.z = r.read_float(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Vector3 {
    fn get_size(&self) -> usize {
        0
        + if self.x == 0f32 { 0 } else { 1 + 4 }
        + if self.y == 0f32 { 0 } else { 1 + 4 }
        + if self.z == 0f32 { 0 } else { 1 + 4 }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.x != 0f32 { w.write_with_tag(13, |w| w.write_float(*&self.x))?; }
        if self.y != 0f32 { w.write_with_tag(21, |w| w.write_float(*&self.y))?; }
        if self.z != 0f32 { w.write_with_tag(29, |w| w.write_float(*&self.z))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Map {
    pub paths: Vec<proto::MapPath>,
}

impl<'a> MessageRead<'a> for Map {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.paths.push(r.read_message::<proto::MapPath>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Map {
    fn get_size(&self) -> usize {
        0
        + self.paths.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.paths { w.write_with_tag(10, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct MapPath {
    pub points: Vec<proto::MapPathPoint>,
}

impl<'a> MessageRead<'a> for MapPath {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.points.push(r.read_message::<proto::MapPathPoint>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for MapPath {
    fn get_size(&self) -> usize {
        0
        + self.points.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.points { w.write_with_tag(10, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct MapPathPoint {
    pub pos: Option<proto::Vector3>,
    pub reach_range: f32,
}

impl<'a> MessageRead<'a> for MapPathPoint {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.pos = Some(r.read_message::<proto::Vector3>(bytes)?),
                Ok(21) => msg.reach_range = r.read_float(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for MapPathPoint {
    fn get_size(&self) -> usize {
        0
        + self.pos.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + if self.reach_range == 0f32 { 0 } else { 1 + 4 }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.pos { w.write_with_tag(10, |w| w.write_message(s))?; }
        if self.reach_range != 0f32 { w.write_with_tag(21, |w| w.write_float(*&self.reach_range))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct MapWave<'a> {
    pub start_time: f32,
    pub units: Vec<proto::MapWaveUnit<'a>>,
}

impl<'a> MessageRead<'a> for MapWave<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(13) => msg.start_time = r.read_float(bytes)?,
                Ok(18) => msg.units.push(r.read_message::<proto::MapWaveUnit>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for MapWave<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.start_time == 0f32 { 0 } else { 1 + 4 }
        + self.units.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.start_time != 0f32 { w.write_with_tag(13, |w| w.write_float(*&self.start_time))?; }
        for s in &self.units { w.write_with_tag(18, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct MapWaveUnit<'a> {
    pub unit_name: Cow<'a, str>,
    pub uint_count: u32,
    pub path_index: u32,
}

impl<'a> MessageRead<'a> for MapWaveUnit<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.unit_name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(16) => msg.uint_count = r.read_uint32(bytes)?,
                Ok(24) => msg.path_index = r.read_uint32(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for MapWaveUnit<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.unit_name == "" { 0 } else { 1 + sizeof_len((&self.unit_name).len()) }
        + if self.uint_count == 0u32 { 0 } else { 1 + sizeof_varint(*(&self.uint_count) as u64) }
        + if self.path_index == 0u32 { 0 } else { 1 + sizeof_varint(*(&self.path_index) as u64) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.unit_name != "" { w.write_with_tag(10, |w| w.write_string(&**&self.unit_name))?; }
        if self.uint_count != 0u32 { w.write_with_tag(16, |w| w.write_uint32(*&self.uint_count))?; }
        if self.path_index != 0u32 { w.write_with_tag(24, |w| w.write_uint32(*&self.path_index))?; }
        Ok(())
    }
}

