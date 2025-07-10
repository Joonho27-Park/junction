# Useful concepts for producing interlocking specifications / **인터로킹 명세 작성을 위한 유용한 개념**

### 0. dgraph representation + "table" structure (underlying route model) / **dgraph 표현 + "테이블" 구조 (기본 경로 모델)**

```text
struct DGraph   { node-a :Node, node-b :Node }
struct Location { node   :Node }      // DNode + direction
```

* **emit-table() / 테이블 내보내기()**
* **emit-route { start = x, end = x, … } / 경로 내보내기 { start = x, end = x, … }**

**objects / 객체**

* dnode
* partnode/location (partnode = dnode + dir) / **부분 노드/위치 (부분 노드 = dnode + 방향)**
* path / **경로**
* area / **영역**

---

### 1. building blocks for imperative specification / **명령형 명세를 위한 빌딩 블록**

*(설명 추가 필요 시 이곳에 작성)*

---

### 2. building blocks for declarative specification / **선언형 명세를 위한 빌딩 블록**

#### a. next-relation on filtered graph / **필터링된 그래프에서 next 관계**

```moonscript
let mainsignal = function(x) do
  return x.type == signal and x.function == "main"
end

let mainsig = function(a,b,d) do
  return a.is(mainsignal) and b.is(mainsignal)
end

neighbors(mainsig)
```

---

### 3. train route / shunting route / **본선 경로 / 입환 경로**

**custom language / 사용자 정의 언어**

```text
routes := { Route(a,b) |
            a <- model.get(mainsignal),
            b <- a.next(mainsignal) }
```

**moonscript**

```moonscript
routes = [ { start = a, end = b, path = p }
           for a in model.get(mainsignal)
           for (b,p) in a.next(mainsignal + samedir(a.dir)) ]

for r in routes
  r.sections = { sec for sec in il:section(model)
                 when sec.intersect(path) }
  r.facingswitches = { sw for sw in model:filter(switch)
                       when path.contains(sw) and sw. }
```

```text
emit-table(route)
for r in routes: emit-route(r)
  -- tries to interpret r table into Route struct on Rust side.
  -- produces warning for unknown fields?
```

---

### 4. repl and custom syntax? / **REPL 및 사용자 정의 문법?**

```text
s1 = model.get(mainsignal).first()

s1
```