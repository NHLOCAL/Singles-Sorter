//! Shared product format dispatch. Stream transformers own all tag policy;
//! this wrapper only selects a format and enforces its publication boundary.
use crate::{
    staged_flac::FlacStagedPlan,
    staged_id3_container::ContainerId3StagedPlan,
    staged_mp4::Mp4StagedPlan,
    staged_ogg::OggStagedPlan,
    staged_tags::{Id3StagedPlan, StagedTagEvidence, TagWriteError},
    staged_wavpack::WavPackStagedPlan,
};
use sorter_core::fix_plan::FieldRepair;
use std::{
    io::{Read, Seek, Write},
    sync::atomic::AtomicBool,
};

#[derive(Clone, Debug)]
pub struct StagedRepairPlan(FormatPlan);

#[derive(Clone, Debug)]
enum FormatPlan {
    Mp3(Id3StagedPlan),
    Flac(FlacStagedPlan),
    Ogg(OggStagedPlan),
    Mp4(Mp4StagedPlan),
    WavPack(WavPackStagedPlan),
    Id3Container(ContainerId3StagedPlan),
}

impl StagedRepairPlan {
    pub fn inspect(
        filename: &str,
        input: &mut (impl Read + Seek),
        source_length: u64,
        repairs: &[FieldRepair],
        cancel: &AtomicBool,
    ) -> Result<Self, TagWriteError> {
        let filename = filename.to_ascii_lowercase();
        if filename.ends_with(".mp3") {
            Id3StagedPlan::inspect(input, source_length, repairs, cancel)
                .map(|plan| Self(FormatPlan::Mp3(plan)))
        } else if filename.ends_with(".flac") {
            let plan = FlacStagedPlan::inspect(input, source_length, repairs, cancel)?;
            plan.check_product_preservation()?;
            Ok(Self(FormatPlan::Flac(plan)))
        } else if filename.ends_with(".ogg") || filename.ends_with(".opus") {
            OggStagedPlan::inspect(input, source_length, repairs, cancel)
                .map(|plan| Self(FormatPlan::Ogg(plan)))
        } else if filename.ends_with(".m4a") || filename.ends_with(".alac") {
            Mp4StagedPlan::inspect(input, source_length, repairs, cancel)
                .map(|plan| Self(FormatPlan::Mp4(plan)))
        } else if filename.ends_with(".wv") {
            WavPackStagedPlan::inspect(input, source_length, repairs, cancel)
                .map(|plan| Self(FormatPlan::WavPack(plan)))
        } else if [".wav", ".aiff", ".dsf"]
            .iter()
            .any(|extension| filename.ends_with(extension))
        {
            ContainerId3StagedPlan::inspect(input, source_length, repairs, cancel)
                .map(|plan| Self(FormatPlan::Id3Container(plan)))
        } else {
            Err(TagWriteError::Unsupported(
                "physical tag format is not supported",
            ))
        }
    }

    pub fn changes_bytes(&self) -> bool {
        match &self.0 {
            FormatPlan::Mp3(plan) => plan.changes_bytes(),
            FormatPlan::Flac(plan) => plan.changes_bytes(),
            FormatPlan::Ogg(plan) => plan.changes_bytes(),
            FormatPlan::Mp4(plan) => plan.changes_bytes(),
            FormatPlan::WavPack(plan) => plan.changes_bytes(),
            FormatPlan::Id3Container(plan) => plan.changes_bytes(),
        }
    }

    pub fn staged_length(&self) -> u64 {
        match &self.0 {
            FormatPlan::Mp3(plan) => plan.staged_length(),
            FormatPlan::Flac(plan) => plan.staged_length(),
            FormatPlan::Ogg(plan) => plan.staged_length(),
            FormatPlan::Mp4(plan) => plan.staged_length(),
            FormatPlan::WavPack(plan) => plan.staged_length(),
            FormatPlan::Id3Container(plan) => plan.staged_length(),
        }
    }

    pub fn write_staged(
        &self,
        input: &mut (impl Read + Seek),
        output: &mut impl Write,
        expected_source_sha256: &str,
        cancel: &AtomicBool,
    ) -> Result<StagedTagEvidence, TagWriteError> {
        match &self.0 {
            FormatPlan::Mp3(plan) => {
                plan.write_staged(input, output, expected_source_sha256, cancel)
            }
            FormatPlan::Flac(plan) => {
                plan.write_staged(input, output, expected_source_sha256, cancel)
            }
            FormatPlan::Ogg(plan) => {
                plan.write_staged(input, output, expected_source_sha256, cancel)
            }
            FormatPlan::Id3Container(plan) => {
                plan.write_staged(input, output, expected_source_sha256, cancel)
            }
            FormatPlan::WavPack(plan) => {
                plan.write_staged(input, output, expected_source_sha256, cancel)
            }
            FormatPlan::Mp4(plan) => {
                plan.write_staged(input, output, expected_source_sha256, cancel)
            }
        }
    }
}
