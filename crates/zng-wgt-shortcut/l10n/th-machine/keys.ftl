### Machine translated by `cargo zng l10n`, 0b1afb2b50bb026726c0531d85ae2f8c2ce81c3ae17e93445959e2db7ce271bc

### สร้างอัตโนมัติโดย `cargo zng l10n`

### ชื่อคีย์ท่าทาง (Gesture) ที่ถูกต้อง
### 
### * ID คือชื่อตัวแปร `Key` [1]
### * ต้องระบุข้อความทั่วไปของ OS โดยสามารถตั้งค่าข้อความเฉพาะของ OS เป็นแอตทริบิวต์ได้
### * แอตทริบิวต์ OS คือค่า `std::env::consts::OS` [2]
### * L10n ไม่รวม Char, Str, modifiers และคีย์ผสม
### 
### หมายเหตุ: ไฟล์นี้ไม่รวมคีย์ที่ถูกต้องทั้งหมด โปรดดูรายการเต็มที่ [1]
### 
### [1]: https://zng-ui.github.io/doc/zng/keyboard/enum.Key.html
### [2]: https://doc.rust-lang.org/std/env/consts/constant.OS.html

ArrowDown = ↓

ArrowLeft = ←

ArrowRight = →

ArrowUp = ↑

Backspace = ←Backspace
    .macos = Delete

Close = ปิด

ContextMenu = ≣เมนูบริบท

Copy = คัดลอก

Cut = ตัด

Delete = ลบ
    .macos = ลบไปข้างหน้า

Eject = ⏏ดีดออก

Enter = ↵ตกลง
    .macos = ↵ย้อนกลับ

Escape = Esc

Find = ค้นหา

Help = ?ช่วยเหลือ

New = ใหม่

Open = เปิด

PageDown = PgDn

PageUp = PgUp

Paste = วาง

PrintScreen = PrtSc

Redo = ทำซ้ำ

Save = บันทึก

Tab = ⭾Tab

Undo = เลิกทำ

ZoomIn = +ขยาย

ZoomOut = -ย่อ