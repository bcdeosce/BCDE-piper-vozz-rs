pub fn samples_to_wav_bytes(samples: &[i16], sr: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut w = Vec::with_capacity(44 + data_len as usize);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data_len).to_le_bytes());
    w.extend_from_slice(b"WAVE");
    w.extend_from_slice(b"fmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&sr.to_le_bytes());
    w.extend_from_slice(&(sr * 2).to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_len.to_le_bytes());
    for s in samples { w.extend_from_slice(&s.to_le_bytes()); }
    w
}
pub fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples.iter().map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16).collect()
}


#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn wav_header_valido() {
        let pcm = vec![0i16; 100];
        let wav = samples_to_wav_bytes(&pcm, 22050);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");
        // 44 bytes de header + 100 amostras × 2 bytes
        assert_eq!(wav.len(), 44 + 200);
        // Sample rate little-endian em offset 24
        let sr = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
        assert_eq!(sr, 22050);
    }

    #[test]
    fn f32_to_i16_satura() {
        let amostras = vec![-2.0f32, -1.0, 0.0, 0.5, 1.0, 2.0];
        let convertido = f32_to_i16(&amostras);
        assert_eq!(convertido[0], -32767);   // clampa
        assert_eq!(convertido[1], -32767);
        assert_eq!(convertido[2], 0);
        assert!(convertido[3] > 0);
        assert_eq!(convertido[4], 32767);
        assert_eq!(convertido[5], 32767);    // clampa
    }

    #[test]
    fn wav_vazio() {
        let wav = samples_to_wav_bytes(&[], 16000);
        assert_eq!(wav.len(), 44);
    }
}
