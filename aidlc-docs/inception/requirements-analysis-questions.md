# Requirements Analysis — Câu hỏi làm rõ

Vui lòng trả lời bằng cách điền chữ cái vào sau thẻ `[Answer]:`.

---

## Question 1 — Extension: Security Baseline
Có muốn áp dụng Security Baseline rules như là hard constraints không?

A) Có — enforce tất cả SECURITY rules (khuyến nghị cho production app tài chính)

B) Không — bỏ qua (phù hợp cho PoC hoặc prototype)

X) Other (please describe after [Answer]: tag below)

[Answer]: 
A
---

## Question 2 — Extension: Resiliency Baseline
Có muốn áp dụng Resiliency Baseline (AWS Well-Architected Framework) không?

A) Có — áp dụng như directional best practices (khuyến nghị cho workload business-critical)

B) Không — bỏ qua

X) Other (please describe after [Answer]: tag below)

[Answer]: 
A
---

## Question 3 — Extension: Property-Based Testing
Có muốn enforce Property-Based Testing rules không?

A) Có — enforce tất cả PBT rules (khuyến nghị cho project có business logic phức tạp)

B) Partial — chỉ enforce cho pure functions và serialization

C) Không — bỏ qua (phù hợp cho CRUD đơn giản)

X) Other (please describe after [Answer]: tag below)

[Answer]: 
A
---

## Question 4 — Vấn đề #1: Datetime library
Tài liệu hiện có mâu thuẫn giữa `time` crate (implementation-plan.md) và `chrono` crate (libs-index.md, libs-sqlx.md). Dự án sẽ dùng library nào?

A) `chrono 0.4` — phổ biến hơn, nhiều ví dụ hơn, đã có sẵn trong hầu hết examples

B) `time 0.3` — nhẹ hơn, được sqlx team khuyến nghị cho sqlx 0.8

X) Other (please describe after [Answer]: tag below)

[Answer]: 
X: Tự quyết định xem dùng thư viện nào phù hợp
---

## Question 5 — Vấn đề #2: Claims struct
Hai tài liệu dùng hai cách định nghĩa Claims khác nhau. Cách nào sẽ được chuẩn hóa?

A) Claims với `user_id: Uuid` riêng biệt (theo `libs-auth.md`) — type-safe hơn

B) Claims chỉ có `sub: String` rồi parse Uuid khi cần (theo `implementation-references.md`) — nhẹ hơn

X) Other (please describe after [Answer]: tag below)

[Answer]: 
X: Tự quyết định xem dùng thư viện nào phù hợp
---

## Question 6 — Oracle trong v1
Oracle database sẽ được xử lý như thế nào trong phiên bản 1?

A) Implement đầy đủ với `oracle` crate native

B) Chỉ implement stub/mock — báo lỗi "not yet supported" khi gọi

C) Bỏ hẳn khỏi scope v1, chỉ support PostgreSQL + MySQL

X) Other (please describe after [Answer]: tag below)

[Answer]: 
A
---

Vui lòng điền câu trả lời rồi cho tôi biết khi xong. Tôi sẽ tiến hành cập nhật tài liệu và lên kế hoạch workflow tiếp theo.
