fn main() {
    // tauri_build::build() zaten tauri.conf.json'daki bundle.icon listesine
    // bakarak Windows exe'sine ikonu ve VERSION resource'unu otomatik gömüyor.
    // Burada ayrıca winres ile elle ikinci bir resource gömmeye çalışmak,
    // linker aşamasında "CVT1100: duplicate resource type:VERSION" hatasına
    // (LNK1123) yol açıyordu. Bu yüzden manuel winres adımı kaldırıldı.
    tauri_build::build();
}
