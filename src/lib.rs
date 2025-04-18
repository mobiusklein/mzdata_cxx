use std::io;
use std::pin::Pin;

use mzdata::prelude::*;

use mzdata::params::{
    ControlledVocabulary as ControlledVocabularyImpl, Param as ParamImpl, ParamValueParseError,
    CURIE as CURIEImpl,
};
use mzdata::spectrum::{
    Acquisition as AcquisitionImpl, IsolationWindow as IsolationWindowImpl,
    MultiLayerIonMobilityFrame as IonMobilityFrameImpl, Precursor as PrecursorImpl,
    ScanEvent as ScanEventImpl, SelectedIon as SelectedIonImpl, Spectrum as SpectrumImpl,
};

use cxx::{CxxString, CxxVector};

macro_rules! result_bool {
    ($op:expr, $out:ident) => {
        if let Some(val) = $op.ok() {
            *$out = val;
            return true;
        } else {
            return false;
        }
    };
}

macro_rules! option_bool {
    ($op:expr, $out:ident) => {
        if let Some(val) = $op {
            *$out = val;
            return true;
        } else {
            return false;
        }
    };
}

macro_rules! option_box_or_err {
    ($op:expr, $err:literal) => {
        match $op {
            Some(item) => Ok(Box::new(item)),
            None => Err($err.into()),
        }
    };
    ($op:expr, $err:expr) => {
        match $op {
            Some(item) => Ok(Box::new(item)),
            None => Err($err),
        }
    };
}

macro_rules! param_methods {
    () => {
        pub fn param(&self, index: usize) -> Result<Box<Param>, String> {
            option_box_or_err!(
                self.0.params().get(index).cloned().map(Param),
                "Parameter not found"
            )
        }

        pub fn params(&self) -> Vec<Param> {
            self.0.params().iter().cloned().map(|p| Param(p)).collect()
        }

        pub fn get_param_by_curie(&self, curie: &ffi::CURIE) -> Result<Box<Param>, String> {
            let params = self.0.params();
            if let Some(val) = params
                .get_param_by_curie(&(*curie).into())
                .map(|p| Param(p.clone()))
            {
                Ok(Box::new(val))
            } else {
                Err(format!("{} not found", CURIEImpl::from(*curie)))
            }
        }
    };
}

pub struct MZReader(mzdata::MZReader<std::fs::File>);

impl MZReader {
    pub fn open(path: &str) -> io::Result<Box<Self>> {
        mzdata::MZReader::open_path(path)
            .inspect_err(|e| eprintln!("Open failed: {e}"))
            .map(|this| Box::new(Self(this)))
    }

    pub fn next(&mut self) -> Result<Box<Spectrum>, &'static str> {
        let spec = self.0.next().map(Spectrum);

        option_box_or_err!(spec, "Failed to read next spectrum")
    }

    pub fn get_by_index(&mut self, index: usize) -> Result<Box<Spectrum>, String> {
        option_box_or_err!(
            self.0.get_spectrum_by_index(index).map(Spectrum),
            format!("index {index} not found")
        )
    }

    pub fn size(&self) -> usize {
        self.0.len()
    }
}

pub fn open(path: &str) -> io::Result<Box<MZReader>> {
    MZReader::open(path)
}

#[derive(Debug, Clone)]
pub struct SelectedIon(SelectedIonImpl);

impl SelectedIon {
    param_methods!();
}

impl IonProperties for SelectedIon {
    #[inline]
    fn mz(&self) -> f64 {
        <SelectedIonImpl as IonProperties>::mz(&self.0)
    }

    #[inline]
    fn neutral_mass(&self) -> f64 {
        <SelectedIonImpl as IonProperties>::neutral_mass(&self.0)
    }

    #[inline]
    fn charge(&self) -> Option<i32> {
        <SelectedIonImpl as IonProperties>::charge(&self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Precursor<'a>(&'a PrecursorImpl);

impl Precursor<'_> {
    pub fn selected_mz(&self, value: &mut f64) -> bool {
        option_bool!(self.0.ions.first().map(|i| i.mz), value)
    }

    pub fn selected_charge(&self, value: &mut i32) -> bool {
        option_bool!(self.0.ions.first().and_then(|i| i.charge), value)
    }

    pub fn selected_ion_mobility(&self, value: &mut f64) -> bool {
        option_bool!(self.0.ions.first().and_then(|i| i.ion_mobility()), value)
    }

    pub fn isolation_window(&self) -> Result<Box<IsolationWindow>, &'static str> {
        match self.0.isolation_window.flags {
            mzdata::spectrum::IsolationWindowState::Unknown => Err("No isolation window found"),
            _ => Ok(Box::new(IsolationWindow(self.0.isolation_window.clone()))),
        }
    }

    pub fn activation_energy(&self, value: &mut f32) -> bool {
        *value = self.0.activation.energy;
        true
    }

    pub fn activation_method_is_combined(&self) -> bool {
        self.0.activation.is_combined()
    }

    pub fn activation_methods(&self) -> Vec<ffi::CURIE> {
        self.0
            .activation
            .methods()
            .iter()
            .map(|method| {
                let acc = method.accession();
                let cv = method.controlled_vocabulary();
                let c = ffi::CURIE::from(CURIEImpl::new(cv, acc));
                c
            })
            .collect()
    }

    pub fn activation_method(&self, value: &mut ffi::CURIE) -> bool {
        if let Some(method) = self.0.activation.method() {
            let acc = method.accession();
            let cv = method.controlled_vocabulary();
            *value = ffi::CURIE::from(CURIEImpl::new(cv, acc));
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct Acquisition<'a>(&'a AcquisitionImpl);

impl<'a> Acquisition<'a> {
    pub fn first_scan(&self) -> Result<Box<ScanEvent<'_>>, &'static str> {
        self.0
            .first_scan()
            .map(|s| Box::new(ScanEvent(s)))
            .ok_or("Scan not found")
    }

    pub fn scan(&self, index: usize) -> Result<Box<ScanEvent<'_>>, &'static str> {
        self.0
            .scans
            .get(index)
            .map(|s| Box::new(ScanEvent(s)))
            .ok_or("Scan not found")
    }

    pub fn instrument_configuration_ids(&self) -> Vec<u32> {
        self.0.instrument_configuration_ids()
    }

    pub fn start_time(&self) -> f64 {
        self.0.start_time()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    param_methods!();
}

#[derive(Debug, Clone)]
pub struct ScanEvent<'a>(&'a ScanEventImpl);

impl<'a> ScanEvent<'a> {
    param_methods!();

    pub fn start_time(&self) -> f64 {
        self.0.start_time
    }

    pub fn injection_time(&self) -> f32 {
        self.0.injection_time
    }

    pub fn instrument_configuration_id(&self) -> u32 {
        self.0.instrument_configuration_id
    }

    pub fn has_ion_mobility(&self) -> bool {
        self.0.has_ion_mobility()
    }

    pub fn ion_mobility(&self, value: &mut f64) -> bool {
        if let Some(val) = self.0.ion_mobility() {
            *value = val;
            true
        } else {
            false
        }
    }

    pub fn filter_string(&self, mut out: Pin<&mut CxxString>) -> bool {
        self.0.filter_string().map(|s| {
            out.as_mut().clear();
            out.as_mut().push_str(&s);
            true
        }).unwrap_or_default()
    }

    pub fn scan_configuration(&self, mut out: Pin<&mut CxxString>) -> bool {
        if let Some(val) = self.0.scan_configuration() {
            out.as_mut().push_str(&val.to_string());
            true
        } else {
            false
        }
    }


}

#[derive(Debug, Clone)]
pub struct Spectrum(SpectrumImpl);

impl Spectrum {
    pub fn id(&self) -> &str {
        self.0.id()
    }

    pub fn index(&self) -> usize {
        self.0.index()
    }

    pub fn start_time(&self) -> f64 {
        self.0.start_time()
    }

    pub fn ms_level(&self) -> u8 {
        self.0.ms_level()
    }

    pub fn is_profile(&self) -> bool {
        matches!(
            self.0.signal_continuity(),
            mzdata::spectrum::SignalContinuity::Profile
        )
    }

    pub fn mzs_into(&self, mut container: Pin<&mut CxxVector<f64>>) {
        for pt in self.0.peaks().iter() {
            container.as_mut().push(pt.mz)
        }
    }

    pub fn intensities_into(&self, mut container: Pin<&mut CxxVector<f32>>) {
        for pt in self.0.peaks().iter() {
            container.as_mut().push(pt.intensity)
        }
    }

    pub fn signal_into(
        &self,
        mut mzs_container: Pin<&mut CxxVector<f64>>,
        mut intensities_container: Pin<&mut CxxVector<f32>>,
    ) {
        for pt in self.0.peaks().iter() {
            mzs_container.as_mut().push(pt.mz);
            intensities_container.as_mut().push(pt.intensity);
        }
    }

    pub fn precursor(&self) -> Result<Box<Precursor<'_>>, String> {
        option_box_or_err!(self.0.precursor().map(Precursor), "No precursor found")
    }

    pub fn acquisition(&self) -> Box<Acquisition<'_>> {
        Box::new(Acquisition(self.0.acquisition()))
    }

    param_methods!();
}

#[derive(Debug, Clone)]
pub struct IonMobilityFrame(IonMobilityFrameImpl);

impl IonMobilityFrame {
    pub fn id(&self) -> &str {
        self.0.id()
    }

    pub fn index(&self) -> usize {
        self.0.index()
    }

    pub fn start_time(&self) -> f64 {
        self.0.start_time()
    }

    pub fn ms_level(&self) -> u8 {
        self.0.ms_level()
    }

    pub fn is_profile(&self) -> bool {
        matches!(
            self.0.signal_continuity(),
            mzdata::spectrum::SignalContinuity::Profile
        )
    }

    pub fn ion_mobility_dimension(&self, mut out: Pin<&mut CxxVector<f64>>) -> bool {
        self.0
            .arrays
            .as_ref()
            .map(|arrays| {
                let vals = arrays.ion_mobility_dimension.as_slice();
                for val in vals {
                    out.as_mut().push(*val);
                }
                true
            })
            .unwrap_or_default()
    }

    pub fn signal_at_ion_mobility_index_into(
        &self,
        ion_mobility_index: usize,
        mut mzs_container: Pin<&mut CxxVector<f64>>,
        mut intensities_container: Pin<&mut CxxVector<f32>>,
        ion_mobility: &mut f64,
    ) {
        if let Some(maps) = self.0.arrays.as_ref() {
            if let Some(val) = maps.ion_mobility_dimension.get(ion_mobility_index) {
                *ion_mobility = *val;
            }
            if let Some(arrays) = maps.arrays.get(ion_mobility_index) {
                if let Some(mzs) = arrays.mzs().ok() {
                    for mz in mzs.iter().copied() {
                        mzs_container.as_mut().push(mz);
                    }
                }
                if let Some(ints) = arrays.intensities().ok() {
                    for int in ints.iter().copied() {
                        intensities_container.as_mut().push(int);
                    }
                }
            }
        }
    }

    pub fn precursor(&self) -> Result<Box<Precursor<'_>>, String> {
        option_box_or_err!(self.0.precursor().map(Precursor), "No precursor found")
    }

    pub fn acquisition(&self) -> Box<Acquisition<'_>> {
        Box::new(Acquisition(self.0.acquisition()))
    }

    param_methods!();
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param(ParamImpl);

impl ParamValue for Param {
    fn is_empty(&self) -> bool {
        <ParamImpl as ParamValue>::is_empty(&self.0)
    }

    fn is_i64(&self) -> bool {
        <ParamImpl as ParamValue>::is_i64(&self.0)
    }

    fn is_f64(&self) -> bool {
        <ParamImpl as ParamValue>::is_f64(&self.0)
    }

    fn is_buffer(&self) -> bool {
        <ParamImpl as ParamValue>::is_buffer(&self.0)
    }

    fn is_str(&self) -> bool {
        <ParamImpl as ParamValue>::is_str(&self.0)
    }

    fn to_f64(&self) -> Result<f64, ParamValueParseError> {
        <ParamImpl as ParamValue>::to_f64(&self.0)
    }

    fn to_i64(&self) -> Result<i64, ParamValueParseError> {
        <ParamImpl as ParamValue>::to_i64(&self.0)
    }

    fn to_str(&self) -> std::borrow::Cow<'_, str> {
        <ParamImpl as ParamValue>::to_str(&self.0)
    }

    fn to_buffer(&self) -> Result<std::borrow::Cow<'_, [u8]>, ParamValueParseError> {
        <ParamImpl as ParamValue>::to_buffer(&self.0)
    }

    fn parse<T: std::str::FromStr>(&self) -> Result<T, T::Err> {
        <ParamImpl as ParamValue>::parse(&self.0)
    }

    fn as_bytes(&self) -> std::borrow::Cow<'_, [u8]> {
        <ParamImpl as ParamValue>::as_bytes(&self.0)
    }

    fn as_ref(&self) -> mzdata::params::ValueRef<'_> {
        <ParamImpl as ParamValue>::as_ref(&self.0)
    }

    fn data_len(&self) -> usize {
        <ParamImpl as ParamValue>::data_len(&self.0)
    }

    fn is_boolean(&self) -> bool {
        <ParamImpl as ParamValue>::is_boolean(&self.0)
    }

    fn to_bool(&self) -> Result<bool, ParamValueParseError> {
        <ParamImpl as ParamValue>::to_bool(&self.0)
    }
}

impl ParamLike for Param {
    fn name(&self) -> &str {
        <ParamImpl as ParamLike>::name(&self.0)
    }

    fn value(&self) -> mzdata::params::ValueRef {
        <ParamImpl as ParamLike>::value(&self.0)
    }

    fn accession(&self) -> Option<mzdata::params::AccessionIntCode> {
        <ParamImpl as ParamLike>::accession(&self.0)
    }

    fn controlled_vocabulary(&self) -> Option<mzdata::params::ControlledVocabulary> {
        <ParamImpl as ParamLike>::controlled_vocabulary(&self.0)
    }

    fn unit(&self) -> mzdata::params::Unit {
        <ParamImpl as ParamLike>::unit(&self.0)
    }
}

impl Param {
    pub fn name(&self) -> &str {
        self.0.name()
    }

    pub fn curie(&self, value: &mut ffi::CURIE) -> bool {
        if let Some(val) = self.0.curie() {
            *value = ffi::CURIE::from(val);
            true
        } else {
            false
        }
    }

    pub fn is_controlled(&self) -> bool {
        self.0.is_controlled()
    }

    pub fn controlled_vocabulary(&self, value: &mut ffi::ControlledVocabulary) -> bool {
        match self.0.controlled_vocabulary.map(|c| c.into()) {
            Some(x) => {
                *value = x;
                true
            }
            None => false,
        }
    }

    pub fn to_bool(&self, value: &mut bool) -> bool {
        result_bool!(self.0.to_bool(), value);
    }

    pub fn to_str(&self, value: Pin<&mut CxxString>) -> bool {
        let out = self.0.to_string();
        value.push_str(&out);
        true
    }

    pub fn to_f64(&self, value: &mut f64) -> bool {
        result_bool!(self.0.to_f64(), value);
    }

    pub fn to_i64(&self, value: &mut i64) -> bool {
        result_bool!(self.0.to_i64(), value);
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct IsolationWindow(IsolationWindowImpl);

impl IsolationWindow {
    pub fn contains(&self, point: f32) -> bool {
        self.0.contains(point)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn target(&self) -> f32 {
        self.0.target as f32
    }

    pub fn lower_bound(&self) -> f32 {
        self.0.lower_bound
    }

    pub fn upper_bound(&self) -> f32 {
        self.0.upper_bound
    }
}

impl From<ffi::ControlledVocabulary> for ControlledVocabularyImpl {
    fn from(value: ffi::ControlledVocabulary) -> Self {
        match value {
            ffi::ControlledVocabulary::MS => Self::MS,
            ffi::ControlledVocabulary::UO => Self::UO,
            ffi::ControlledVocabulary::EFO => Self::EFO,
            ffi::ControlledVocabulary::OBI => Self::OBI,
            ffi::ControlledVocabulary::HANCESTRO => Self::HANCESTRO,
            ffi::ControlledVocabulary::BFO => Self::BFO,
            ffi::ControlledVocabulary::NCIT => Self::NCIT,
            ffi::ControlledVocabulary::BTO => Self::BTO,
            ffi::ControlledVocabulary::PRIDE => Self::PRIDE,
            ffi::ControlledVocabulary::Unknown => Self::Unknown,
            _ => Self::Unknown,
        }
    }
}

impl From<ControlledVocabularyImpl> for ffi::ControlledVocabulary {
    fn from(value: ControlledVocabularyImpl) -> ffi::ControlledVocabulary {
        match value {
            ControlledVocabularyImpl::MS => ffi::ControlledVocabulary::MS,
            ControlledVocabularyImpl::UO => ffi::ControlledVocabulary::UO,
            ControlledVocabularyImpl::EFO => ffi::ControlledVocabulary::EFO,
            ControlledVocabularyImpl::OBI => ffi::ControlledVocabulary::OBI,
            ControlledVocabularyImpl::HANCESTRO => ffi::ControlledVocabulary::HANCESTRO,
            ControlledVocabularyImpl::BFO => ffi::ControlledVocabulary::BFO,
            ControlledVocabularyImpl::NCIT => ffi::ControlledVocabulary::NCIT,
            ControlledVocabularyImpl::BTO => ffi::ControlledVocabulary::BTO,
            ControlledVocabularyImpl::PRIDE => ffi::ControlledVocabulary::PRIDE,
            ControlledVocabularyImpl::Unknown => ffi::ControlledVocabulary::Unknown,
        }
    }
}

impl From<CURIEImpl> for ffi::CURIE {
    fn from(value: CURIEImpl) -> Self {
        Self {
            controlled_vocabulary: value.controlled_vocabulary.into(),
            accession: value.accession,
        }
    }
}

impl From<ffi::CURIE> for CURIEImpl {
    fn from(value: ffi::CURIE) -> Self {
        Self {
            controlled_vocabulary: value.controlled_vocabulary.into(),
            accession: value.accession,
        }
    }
}

pub fn curie_to_string(curie: &ffi::CURIE) -> String {
    CURIEImpl::from(*curie).to_string()
}

#[derive(Clone)]
pub struct ParameterContainer<'a>(&'a dyn ParamDescribedRead);

impl<'a> ParameterContainer<'a> {
    param_methods!();
}

#[cxx::bridge(namespace = "mzdata_cpp")]
pub(crate) mod ffi {

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ControlledVocabulary {
        MS,
        UO,
        EFO,
        OBI,
        HANCESTRO,
        BFO,
        NCIT,
        BTO,
        PRIDE,
        Unknown,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CURIE {
        pub controlled_vocabulary: ControlledVocabulary,
        pub accession: u32,
    }

    extern "Rust" {
        pub type Param;

        pub fn name(self: &Param) -> &str;
        pub fn is_controlled(self: &Param) -> bool;
        pub fn controlled_vocabulary(self: &Param, value: &mut ControlledVocabulary) -> bool;
        pub fn to_bool(self: &Param, value: &mut bool) -> bool;
        pub fn to_str(self: &Param, value: Pin<&mut CxxString>) -> bool;
        pub fn to_f64(self: &Param, value: &mut f64) -> bool;
        pub fn to_i64(self: &Param, value: &mut i64) -> bool;
        pub fn curie(self: &Param, value: &mut CURIE) -> bool;
    }

    extern "Rust" {
        pub type IsolationWindow;

        pub fn contains(&self, point: f32) -> bool;
        pub fn is_empty(&self) -> bool;
        pub fn target(&self) -> f32;
        pub fn lower_bound(&self) -> f32;
        pub fn upper_bound(&self) -> f32;
    }

    extern "Rust" {
        pub type Precursor<'a>;

        pub fn selected_mz(&self, value: &mut f64) -> bool;
        pub fn selected_charge(&self, value: &mut i32) -> bool;
        pub fn selected_ion_mobility(&self, value: &mut f64) -> bool;
        pub fn isolation_window(&self) -> Result<Box<IsolationWindow>>;
        pub fn activation_energy(&self, value: &mut f32) -> bool;
        pub fn activation_method_is_combined(&self) -> bool;
        pub fn activation_methods(&self) -> Vec<CURIE>;
        pub fn activation_method(&self, value: &mut CURIE) -> bool;
    }

    extern "Rust" {
        pub type ScanEvent<'a>;

        pub fn start_time(&self) -> f64;
        pub fn injection_time(&self) -> f32;
        pub fn instrument_configuration_id(&self) -> u32;
        pub fn ion_mobility(&self, value: &mut f64) -> bool;
        pub fn has_ion_mobility(&self) -> bool;

        pub fn scan_configuration(&self, mut out: Pin<&mut CxxString>) -> bool;
        pub fn filter_string(&self, mut out: Pin<&mut CxxString>) -> bool;

        pub fn param(&self, index: usize) -> Result<Box<Param>>;
        pub fn params(&self) -> Vec<Param>;
        pub fn get_param_by_curie(&self, curie: &CURIE) -> Result<Box<Param>>;
    }

    extern "Rust" {
        pub type Acquisition<'a>;

        pub unsafe fn first_scan<'a>(&'a self) -> Result<Box<ScanEvent<'a>>>;
        pub unsafe fn scan<'a>(&'a self, index: usize) -> Result<Box<ScanEvent<'a>>>;
        pub fn instrument_configuration_ids(&self) -> Vec<u32>;
        pub fn start_time(&self) -> f64;
        pub fn len(&self) -> usize;

        pub fn param(&self, index: usize) -> Result<Box<Param>>;
        pub fn params(&self) -> Vec<Param>;
        pub fn get_param_by_curie(&self, curie: &CURIE) -> Result<Box<Param>>;
    }

    extern "Rust" {
        pub type Spectrum;

        pub fn mzs_into(&self, mut container: Pin<&mut CxxVector<f64>>);
        pub fn intensities_into(&self, mut container: Pin<&mut CxxVector<f32>>);
        pub fn signal_into(
            &self,
            mut mzs_container: Pin<&mut CxxVector<f64>>,
            mut intensities_container: Pin<&mut CxxVector<f32>>,
        );

        pub fn id(&self) -> &str;
        pub fn index(&self) -> usize;
        pub fn start_time(&self) -> f64;
        pub fn ms_level(&self) -> u8;
        pub fn is_profile(&self) -> bool;
        pub unsafe fn precursor<'a>(&'a self) -> Result<Box<Precursor<'a>>>;
        pub unsafe fn acquisition<'a>(&'a self) -> Box<Acquisition<'a>>;

        pub fn param(&self, index: usize) -> Result<Box<Param>>;
        pub fn params(&self) -> Vec<Param>;
        pub fn get_param_by_curie(&self, curie: &CURIE) -> Result<Box<Param>>;
    }

    extern "Rust" {
        pub type IonMobilityFrame;

        pub fn id(&self) -> &str;
        pub fn index(&self) -> usize;
        pub fn start_time(&self) -> f64;
        pub fn ms_level(&self) -> u8;
        pub fn is_profile(&self) -> bool;
        pub unsafe fn precursor<'a>(&'a self) -> Result<Box<Precursor<'a>>>;

        pub fn ion_mobility_dimension(&self, mut out: Pin<&mut CxxVector<f64>>) -> bool;

        pub fn param(&self, index: usize) -> Result<Box<Param>>;
        pub fn params(&self) -> Vec<Param>;
        pub fn get_param_by_curie(&self, curie: &CURIE) -> Result<Box<Param>>;

        pub fn signal_at_ion_mobility_index_into(
            &self,
            ion_mobility_index: usize,
            mut mzs_container: Pin<&mut CxxVector<f64>>,
            mut intensities_container: Pin<&mut CxxVector<f32>>,
            ion_mobility: &mut f64,
        );
    }

    extern "Rust" {
        pub type MZReader;

        pub fn open(path: &str) -> Result<Box<MZReader>>;

        pub fn size(&self) -> usize;
        pub fn next(&mut self) -> Result<Box<Spectrum>>;
        pub fn get_by_index(&mut self, index: usize) -> Result<Box<Spectrum>>;
    }
}
