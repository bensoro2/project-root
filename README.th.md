# แพลตฟอร์มค้นหาความหมายรีวิวสินค้า (Review Semantic Search Platform)

ระบบค้นหารีวิวตามความหมาย (Semantic Search) ขนาดเบา พัฒนา بالคื้นด้วยภาษา Rust จัดเก็บข้อมูลและเวกเตอร์แบบไฟล์ต่อเนื่อง (append-only) โดยไม่ต้องใช้ฐานข้อมูลภายนอก พร้อมสคริปต์เชื่อมต่อและเปรียบเทียบกับ Qdrant ได้ทันที

## คุณสมบัติเด่น

- **ค้นหาตามความหมาย**: ค้นหารีวิวที่เกี่ยวข้องตามเนื้อหา ไม่ใช่แค่คีย์เวิร์ด
- **ไม่ง้อฐานข้อมูล**: เก็บเวกเตอร์ใน `reviews.index` และเมทาดาต้าใน `reviews.jsonl`
- **Rust เต็มระบบ**: Backend (Axum), Frontend (Leptos) และ Vector Search (spfresh) ล้วนเขียนด้วย Rust
- **รองรับ Embedding**: ใช้ [fastembed-rs](https://crates.io/crates/fastembed) สร้างเวกเตอร์ 128 มิติ
- **Docker พร้อมใช้**: มี `docker-compose.yml` และ Dockerfile สำหรับการดีพลอย
- **สถาปัตยกรรมยืดหยุ่น**: สลับเอนจินเวกเตอร์ได้ง่าย พร้อมสคริปต์สำหรับ Qdrant

## โครงสร้างโปรเจกต์ (ย่อ)

```
project-root/
├── backend/      # เว็บเซิร์ฟเวอร์ Axum + fastembed + spfresh
│   ├── data/     # ไฟล์ข้อมูลถาวร
│   └── src/...   # โค้ดหลัก
├── frontend/     # Leptos SPA (WASM)
└── docker-compose.yml
```

## การรันแบบเนทีฟ

```bash
# โหมด dev (ใช้ dummy embedder)
cargo run --manifest-path backend/Cargo.toml

# โหมดจริง (โหลดโมเดล ~80 MB ครั้งแรก)
cargo run --manifest-path backend/Cargo.toml --features fastembed
```

เซิร์ฟเวอร์เปิดที่ `http://localhost:8000` และบันทึกไฟล์ไว้ที่ `backend/data/`.

## ตั้งค่า Qdrant เพื่อเปรียบเทียบ

### 1️⃣ เริ่ม Qdrant (Docker)
```bash
docker run -p 6333:6333 --name qdrant -d qdrant/qdrant
```

### 2️⃣ อัปโหลดเวกเตอร์เข้า Qdrant
```bash
# สร้างไบนารี release (fastembed + spfresh)
cargo build --release --manifest-path backend/Cargo.toml --features "fastembed spfresh"

# อัปโหลดรีวิว 1,000 รายการ
cargo run --release --features "fastembed spfresh" --bin qdrant_loader
```

### 3️⃣ รันเบนช์มาร์ก
```bash
cargo run --release --features "fastembed spfresh" --bin bench_compare
```

ผลลัพธ์ตัวอย่าง
```
=== Benchmark Results ===
Avg latency (µs)  - spfresh: 18726.53, qdrant: 3133.12
Avg recall@10      : 0.891
```

### สรุปผลทดสอบ

| เอนจิน | เวลาเฉลี่ย (µs) | Recall@10 |
|---------|-----------------|-----------|
| spfresh | 18 727          | 0.891     |
| Qdrant  | 3 133           | 0.891     |

Qdrant (HNSW) ตอบสนองเร็วกว่า ~6 เท่า ในขณะที่ความแม่นยำเท่ากัน เลือกใช้ตามบริบท:
- **spfresh**: ฝังรวมในแอป, ไม่พึ่งบริการภายนอก
- **Qdrant**: สเกลแยก, รองรับกรอง metadata, replication, persistence

### การวิเคราะห์ผลลัพธ์การเปรียบเทียบ TOP_K = 10 และ TOP_K = 100

จากการทดลองเปรียบเทียบประสิทธิภาพระหว่าง SPFresh และ Qdrant ด้วยค่า TOP_K = 100 และ SAMPLE_QUERIES = 1000 พบว่า:

| เอนจิน | เวลาเฉลี่ย (µs) | Recall@100 |
|---------|-----------------|------------|
| spfresh | 9,407.78        | 0.940      |
| Qdrant  | 3,060.23        | 0.940      |

**การเปรียบเทียบกับการทดลอง TOP_K = 10:**
- ในกรณี TOP_K = 10:
  - Qdrant: latency 3,133 µs, recall@10 = 0.891
  - SPFresh: latency 18,727 µs, recall@10 = 0.891
  - Qdrant เร็วกว่าประมาณ 6 เท่า
- ในกรณี TOP_K = 100:
  - Qdrant: latency 3,060.23 µs, recall@100 = 0.940
  - SPFresh: latency 9,407.78 µs, recall@100 = 0.940
  - Qdrant เร็วกว่าประมาณ 3 เท่า

**ข้อสังเกต:**
- เมื่อเพิ่มค่า TOP_K จาก 10 เป็น 100:
  - Latency ของ SPFresh ลดลงอย่างมีนัยสำคัญ (จาก 18,727 µs เป็น 9,407.78 µs)
  - Latency ของ Qdrant แทบไม่เปลี่ยนแปลน (จาก 3,133 µs เป็น 3,060.23 µs)
  - Recall เพิ่มขึ้นทั้งสองระบบ (จาก 0.891 เป็น 0.940)

**สรุป:**
- Qdrant มีประสิทธิภาพที่ดีกว่าในทั้งสองกรณี (TOP_K = 10 และ TOP_K = 100)
- ความแตกต่างของ latency ระหว่างสองระบบลดลงเมื่อ TOP_K เพิ่มขึ้น แต่ Qdrant ยังคงเร็วกว่าอย่างมีนัยสำคัญ
- การเพิ่ม TOP_K ทำให้ recall เพิ่มขึ้น ซึ่งเป็นไปตามที่คาดหวัง

## API หลัก (ย่อ)

1. `POST /reviews`  – เพิ่มรีวิวเดี่ยว
2. `POST /reviews/bulk` – เพิ่มหลายรีวิว
3. `POST /search` – ค้นหาเชิงความหมาย

ดูรายละเอียดตัวอย่าง JSON ได้ในโค้ดหรือ README ภาษาอังกฤษ (README.md)

## พัฒนาและคอนทริบิวต์

1. ติดตั้ง Rust → <https://rust-lang.org/tools/install>
2. โคลนโปรเจกต์แล้วรันคำสั่งข้างต้น
3. PR / issue ยินดีต้อนรับ!

## License

MIT
