use std::fmt::Display;
use std::io;
use std::pin::Pin;

use mzdata::mzpeaks::{CentroidPeak, DeconvolutedPeak};
use mzdata::prelude::*;

use mzdata::params::{
    ControlledVocabulary as ControlledVocabularyImpl, Param as ParamImpl, ParamValueParseError,
    CURIE as CURIEImpl,
};
use mzdata::spectrum::IsolationWindow as IsolationWindowImpl;
use mzdata::spectrum::{
    Precursor as PrecursorImpl, SelectedIon as SelectedIonImpl, Spectrum as SpectrumImpl,
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

pub struct MZReader(mzdata::MZReader<std::fs::File>);

impl MZReader {
    pub fn open(path: &str) -> io::Result<Box<Self>> {
        mzdata::MZReader::open_path(path).map(|this| Box::new(Self(this)))
    }

    pub fn next(&mut self, value: &mut Spectrum) -> bool {
        option_bool!(self.0.next().map(Spectrum), value);
    }

    pub fn get_by_index(&mut self, index: usize, value: &mut Spectrum) -> bool {
        option_bool!(self.0.get_spectrum_by_index(index).map(Spectrum), value);
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

impl ParamDescribed for SelectedIon {
    fn params(&self) -> &[mzdata::params::Param] {
        <SelectedIonImpl as ParamDescribed>::params(&self.0)
    }

    fn params_mut(&mut self) -> &mut mzdata::params::ParamList {
        <SelectedIonImpl as ParamDescribed>::params_mut(&mut self.0)
    }
}

impl IonMobilityMeasure for SelectedIon {}

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

    pub fn isolation_window(&self, value: &mut IsolationWindow) -> bool {
        match self.0.isolation_window.flags {
            mzdata::spectrum::IsolationWindowState::Unknown => false,
            _ => {
                *value = IsolationWindow(self.0.isolation_window.clone());
                true
            }
        }
    }

    pub fn activation_energy(&self, value: &mut f32) -> bool {
        *value = self.0.activation.energy;
        true
    }

    pub fn activation_method_is_combined(&self) -> bool {
        self.0.activation.is_combined()
    }

    pub fn activation_methods(&self) -> Vec<CURIE> {
        self.0.activation.methods().iter().map(|method| {
            let acc = method.accession();
            let cv = method.controlled_vocabulary();
            let c = CURIE(CURIEImpl::new(cv, acc));
            c
        }).collect()
    }

    pub fn activation_method(&self, value: &mut CURIE) -> bool {
        if let Some(method) = self.0.activation.method() {
            let acc = method.accession();
            let cv = method.controlled_vocabulary();
            *value = CURIE(CURIEImpl::new(cv, acc));
            true
        } else {
            false
        }

    }
}

#[derive(Debug, Clone)]
pub struct Spectrum(SpectrumImpl);

impl ParamDescribed for Spectrum {
    fn params(&self) -> &[mzdata::params::Param] {
        <SpectrumImpl as ParamDescribed>::params(&self.0)
    }

    fn params_mut(&mut self) -> &mut mzdata::params::ParamList {
        <SpectrumImpl as ParamDescribed>::params_mut(&mut self.0)
    }
}

impl SpectrumLike for Spectrum {
    #[inline]
    fn description(&self) -> &mzdata::spectrum::SpectrumDescription {
        <SpectrumImpl as SpectrumLike>::description(&self.0)
    }

    fn description_mut(&mut self) -> &mut mzdata::spectrum::SpectrumDescription {
        <SpectrumImpl as SpectrumLike>::description_mut(&mut self.0)
    }

    fn peaks(&'_ self) -> mzdata::spectrum::RefPeakDataLevel<'_, CentroidPeak, DeconvolutedPeak> {
        <SpectrumImpl as SpectrumLike>::peaks(&self.0)
    }

    fn raw_arrays(&'_ self) -> Option<&'_ mzdata::spectrum::BinaryArrayMap> {
        <SpectrumImpl as SpectrumLike>::raw_arrays(&self.0)
    }

    fn into_peaks_and_description(
        self,
    ) -> (
        mzdata::spectrum::PeakDataLevel,
        mzdata::spectrum::SpectrumDescription,
    ) {
        <SpectrumImpl as SpectrumLike>::into_peaks_and_description(self.0)
    }
}

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
        matches!(self.0.signal_continuity(), mzdata::spectrum::SignalContinuity::Profile)
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

    pub fn precursor<'a>(&'a self, value: &mut Precursor<'a>) -> bool {
        option_bool!(self.0.precursor().map(Precursor), value);
    }
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

    pub fn curie(&self, value: &mut CURIE) -> bool {
        if let Some(val) = self.0.curie() {
            *value = CURIE(val);
            true
        } else {
            false
        }
    }

    pub fn is_controlled(&self) -> bool {
        self.0.is_controlled()
    }

    pub fn controlled_vocabulary(&self, value: &mut param_ffi::ControlledVocabulary) -> bool {
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

impl From<param_ffi::ControlledVocabulary> for ControlledVocabularyImpl {
    fn from(value: param_ffi::ControlledVocabulary) -> Self {
        match value {
            param_ffi::ControlledVocabulary::MS => Self::MS,
            param_ffi::ControlledVocabulary::UO => Self::UO,
            param_ffi::ControlledVocabulary::EFO => Self::EFO,
            param_ffi::ControlledVocabulary::OBI => Self::OBI,
            param_ffi::ControlledVocabulary::HANCESTRO => Self::HANCESTRO,
            param_ffi::ControlledVocabulary::BFO => Self::BFO,
            param_ffi::ControlledVocabulary::NCIT => Self::NCIT,
            param_ffi::ControlledVocabulary::BTO => Self::BTO,
            param_ffi::ControlledVocabulary::PRIDE => Self::PRIDE,
            param_ffi::ControlledVocabulary::Unknown => Self::Unknown,
            _ => Self::Unknown,
        }
    }
}

impl From<ControlledVocabularyImpl> for param_ffi::ControlledVocabulary {
    fn from(value: ControlledVocabularyImpl) -> param_ffi::ControlledVocabulary {
        match value {
            ControlledVocabularyImpl::MS => param_ffi::ControlledVocabulary::MS,
            ControlledVocabularyImpl::UO => param_ffi::ControlledVocabulary::UO,
            ControlledVocabularyImpl::EFO => param_ffi::ControlledVocabulary::EFO,
            ControlledVocabularyImpl::OBI => param_ffi::ControlledVocabulary::OBI,
            ControlledVocabularyImpl::HANCESTRO => param_ffi::ControlledVocabulary::HANCESTRO,
            ControlledVocabularyImpl::BFO => param_ffi::ControlledVocabulary::BFO,
            ControlledVocabularyImpl::NCIT => param_ffi::ControlledVocabulary::NCIT,
            ControlledVocabularyImpl::BTO => param_ffi::ControlledVocabulary::BTO,
            ControlledVocabularyImpl::PRIDE => param_ffi::ControlledVocabulary::PRIDE,
            ControlledVocabularyImpl::Unknown => param_ffi::ControlledVocabulary::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CURIE(CURIEImpl);

impl CURIE {
    pub fn controlled_vocabulary(&self) -> param_ffi::ControlledVocabulary {
        self.0.controlled_vocabulary.into()
    }

    pub fn accession(&self) -> u32 {
        self.0.accession
    }

    pub fn as_param(&self) -> Param {
        Param(self.0.as_param())
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl Display for CURIE {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <CURIEImpl as Display>::fmt(&self.0, f)
    }
}

#[cxx::bridge(namespace = "mzdata_cpp")]
pub(crate) mod param_ffi {

    #[derive(Clone, Copy, PartialEq)]
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

    extern "Rust" {
        pub type Param;
        pub type CURIE;

        pub fn controlled_vocabulary(self: &CURIE) -> ControlledVocabulary;
        pub fn accession(self: &CURIE) -> u32;
        pub fn to_string(self: &CURIE) -> String;

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
        pub fn isolation_window(&self, value: &mut IsolationWindow) -> bool;
        pub fn activation_energy(&self, value: &mut f32) -> bool;
        pub fn activation_method_is_combined(&self) -> bool;
        pub fn activation_methods(&self) -> Vec<CURIE>;
        pub fn activation_method(&self, value: &mut CURIE) -> bool;
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
        pub unsafe fn precursor<'a>(&'a self, value: &mut Precursor<'a>) -> bool;
    }

    extern "Rust" {
        pub type MZReader;

        pub fn open(path: &str) -> Result<Box<MZReader>>;

        pub fn size(&self) -> usize;
        pub fn next(&mut self, value: &mut Spectrum) -> bool;
    }
}
