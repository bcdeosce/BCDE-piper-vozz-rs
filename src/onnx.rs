use anyhow::{anyhow, Context, Result};
use ndarray::Array;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Value;
use std::path::Path;

pub struct OnnxModel {
    session: Session,
    num_speakers: u32,
}

impl OnnxModel {
    pub fn load(path: &Path, num_speakers: u32) -> Result<Self> {
        let session = Session::builder()
            .map_err(|e| anyhow!("ort builder: {}", e))?
            .with_intra_threads(1).map_err(|e| anyhow!("{}", e))?
            .with_inter_threads(1).map_err(|e| anyhow!("{}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow!("{}", e))?
            .commit_from_file(path)
            .with_context(|| format!("load {}", path.display()))?;
        Ok(Self { session, num_speakers })
    }

    pub fn num_speakers(&self) -> u32 { self.num_speakers }

    pub fn infer(&mut self, ids: &[i64], scales: [f32; 3], sid: Option<i64>)
        -> Result<Vec<f32>>
    {
        let n = ids.len();
        let input = Array::from_shape_vec((1, n), ids.to_vec())
            .map_err(|e| anyhow!("{}", e))?;
        let input_lengths = Array::from_shape_vec((1,), vec![n as i64])
            .map_err(|e| anyhow!("{}", e))?;
        let scales_arr = Array::from_shape_vec((3,), scales.to_vec())
            .map_err(|e| anyhow!("{}", e))?;

        let input_v = Value::from_array(input).map_err(|e| anyhow!("{}", e))?;
        let len_v   = Value::from_array(input_lengths).map_err(|e| anyhow!("{}", e))?;
        let scales_v = Value::from_array(scales_arr).map_err(|e| anyhow!("{}", e))?;

        let outputs = if self.num_speakers > 1 {
            let sid_arr = Array::from_shape_vec((1,), vec![sid.unwrap_or(0)])
                .map_err(|e| anyhow!("{}", e))?;
            let sid_v = Value::from_array(sid_arr).map_err(|e| anyhow!("{}", e))?;
            self.session.run(ort::inputs![
                "input" => input_v, "input_lengths" => len_v,
                "scales" => scales_v, "sid" => sid_v,
            ]).map_err(|e| anyhow!("run: {}", e))?
        } else {
            self.session.run(ort::inputs![
                "input" => input_v, "input_lengths" => len_v,
                "scales" => scales_v,
            ]).map_err(|e| anyhow!("run: {}", e))?
        };

        let out = outputs.get("output").ok_or_else(|| anyhow!("output ausente"))?;
        let (_s, data) = out.try_extract_tensor::<f32>()
            .map_err(|e| anyhow!("extract: {}", e))?;
        Ok(data.to_vec())
    }
}
