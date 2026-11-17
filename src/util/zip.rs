//! Escritor ZIP mínimo sin compresión (método 0, STORED).
//!
//! Alcanza para OpenDocument, que exige `mimetype` como primera entrada y sin
//! comprimir. shortcut: sin deflate ni ZIP64 (límite 4 GiB), suficiente para
//! hojas de cálculo de una base personal.

const fn tabla_crc() -> [u32; 256] {
    let mut tabla = [0_u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 == 1 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        tabla[n] = c;
        n += 1;
    }
    tabla
}

const TABLA_CRC: [u32; 256] = tabla_crc();

/// CRC-32 (IEEE 802.3), el que usa ZIP.
#[must_use]
pub fn crc32(datos: &[u8]) -> u32 {
    !datos.iter().fold(!0_u32, |crc, byte| {
        TABLA_CRC[((crc ^ u32::from(*byte)) & 0xFF) as usize] ^ (crc >> 8)
    })
}

/// Arma un ZIP en memoria con las entradas en el orden dado.
#[must_use]
pub fn zip_stored(entradas: &[(&str, &[u8])]) -> Vec<u8> {
    // Fecha DOS fija (1980-01-01 00:00): el contenido no depende del reloj.
    const HORA_DOS: u16 = 0;
    const FECHA_DOS: u16 = (1 << 5) | 1;
    let mut salida = Vec::new();
    let mut central = Vec::new();
    for (nombre, datos) in entradas {
        let offset = salida.len() as u32;
        let crc = crc32(datos);
        let tamano = datos.len() as u32;
        let nombre = nombre.as_bytes();
        let comun = |v: &mut Vec<u8>| {
            v.extend_from_slice(&20_u16.to_le_bytes()); // versión necesaria
            v.extend_from_slice(&0x0800_u16.to_le_bytes()); // nombres UTF-8
            v.extend_from_slice(&0_u16.to_le_bytes()); // STORED
            v.extend_from_slice(&HORA_DOS.to_le_bytes());
            v.extend_from_slice(&FECHA_DOS.to_le_bytes());
            v.extend_from_slice(&crc.to_le_bytes());
            v.extend_from_slice(&tamano.to_le_bytes());
            v.extend_from_slice(&tamano.to_le_bytes());
            v.extend_from_slice(&(nombre.len() as u16).to_le_bytes());
            v.extend_from_slice(&0_u16.to_le_bytes()); // extra
        };
        salida.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
        comun(&mut salida);
        salida.extend_from_slice(nombre);
        salida.extend_from_slice(datos);

        central.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
        central.extend_from_slice(&20_u16.to_le_bytes()); // versión creadora
        comun(&mut central);
        central.extend_from_slice(&[0; 6]); // largo de comentario, disco, atributos internos (2+2+2)
        central.extend_from_slice(&0_u32.to_le_bytes()); // atributos externos
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(nombre);
    }
    let inicio_central = salida.len() as u32;
    let total = entradas.len() as u16;
    salida.extend_from_slice(&central);
    salida.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    salida.extend_from_slice(&[0; 4]); // número de disco
    salida.extend_from_slice(&total.to_le_bytes());
    salida.extend_from_slice(&total.to_le_bytes());
    salida.extend_from_slice(&(central.len() as u32).to_le_bytes());
    salida.extend_from_slice(&inicio_central.to_le_bytes());
    salida.extend_from_slice(&0_u16.to_le_bytes()); // comentario
    salida
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_standard_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn archive_structure_round_trips() {
        let zip = zip_stored(&[("mimetype", b"abc"), ("dir/b.xml", b"<x/>")]);
        let u16_en = |i: usize| u16::from_le_bytes([zip[i], zip[i + 1]]);
        let u32_en = |i: usize| u32::from_le_bytes(zip[i..i + 4].try_into().unwrap());
        // Primera entrada: firma, método 0, CRC y nombre.
        assert_eq!(u32_en(0), 0x0403_4b50);
        assert_eq!(u16_en(8), 0);
        assert_eq!(u32_en(14), crc32(b"abc"));
        assert_eq!(&zip[30..38], b"mimetype");
        assert_eq!(&zip[38..41], b"abc");
        // EOCD al final: 2 entradas y el directorio central donde dice.
        let eocd = zip.len() - 22;
        assert_eq!(u32_en(eocd), 0x0605_4b50);
        assert_eq!(u16_en(eocd + 10), 2);
        let central = u32_en(eocd + 16) as usize;
        assert_eq!(u32_en(central), 0x0201_4b50);
    }

    #[test]
    fn every_central_entry_is_where_a_reader_expects_it() {
        let entradas: [(&str, &[u8]); 3] = [
            ("mimetype", b"abc"),
            ("a.xml", b"<a/>"),
            ("META-INF/b.xml", b"<b/>"),
        ];
        let zip = zip_stored(&entradas);
        let u16_en = |i: usize| usize::from(u16::from_le_bytes([zip[i], zip[i + 1]]));
        let u32_en = |i: usize| u32::from_le_bytes(zip[i..i + 4].try_into().unwrap());
        let eocd = zip.len() - 22;
        let mut pos = u32_en(eocd + 16) as usize;
        for (nombre, datos) in entradas {
            // Encabezado central de 46 bytes + nombre + extra + comentario.
            assert_eq!(u32_en(pos), 0x0201_4b50, "firma de {nombre}");
            let largo_nombre = u16_en(pos + 28);
            assert_eq!(&zip[pos + 46..pos + 46 + largo_nombre], nombre.as_bytes());
            let local = u32_en(pos + 42) as usize;
            assert_eq!(u32_en(local), 0x0403_4b50);
            let inicio = local + 30 + u16_en(local + 26) + u16_en(local + 28);
            assert_eq!(&zip[inicio..inicio + datos.len()], datos);
            pos += 46 + largo_nombre + u16_en(pos + 30) + u16_en(pos + 32);
        }
        assert_eq!(pos, eocd, "el directorio central termina justo en el EOCD");
    }
}
