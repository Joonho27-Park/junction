# glrail reboot juni 2019 / glrail 재부트 2019년 6월

## Infrastructure document model / 인프라 문서 모델

Topology = 이산 좌표 선분(discrete‑coord lines)
           + specific nodes' (좌표로 지정된) properties 
              (삭제되거나 연결 차수가 변경되면 소실)
           + 선택적 길이 정보(optional length info)

Objects = 좌표+각도(coord+angle), 선택적 mileage, 선택적 "pos"(?),
          function (TODO 다중 기능, 사용자 정의 도형, 이름/id?)

TODO: (1) polygon areas?(폴리곤 영역?) (2) delimited areas?(경계가 있는 영역?) (3) track properties(선로 속성)

(선분으로부터 노드 유형 파생,
그래픽 위치에서 km / pos 파생(명시된 경우 제외),
도식 + 사용자 정의 데이터 + 경로 파생 + 운행 지령용 DGraph 파생)

## Infrastructure editor / 인프라 편집기

* x  선로 그리기(Draw tracks)
* (?) 객체 배치(Place objects)
* x  선/객체 선택(Select lines / objects)

- x  선/객체 지우기(Erase lines / objects)
- x  선+노드/객체 이동(Move lines+nodes / objects)
- x  선택 항목 컨텍스트 메뉴(Context menu on selection)
  a.  x  노드 수정(Modify nodes) (노드 데이터)
  b. ( ) 객체 수정(Modify objects) (object menu)
  c. ( ) 선로 길이 지정(Lengths on tracks)

* x  스크롤(Scroll) (휠 및 CTRL‑드래그)
* 복사/붙여넣기(Copy/paste) (복사=커서 위치 기준?)

## Static interlocking model / 정적 인터로킹 모델

language:

* assignment/equality(할당/동등)
* objects of type, objects in sets — 유형은 고정된 집합(대문자)
  entities, path, area, and sets
* set builder, Lua‑table 유사 객체 생성

  1. base set
  2. "nested" 바인딩·필터 곱(product)
* filters(멤버십, 동등)
* operations(다중 차수)
* tuples(순서형, 그리고/또는 맵)

유형은 열거 가능(enumerable)과 비열거(non‑enumerable)로 구분되고,
집합은 열거 가능 유형이다.

**toplevel sets/types**:
Location(track+pos) (non‑enumerable)
Entity — 모든 엔티티(enumerable)
Path (non‑enumerable)
Area (non‑enumerable)

미리 정의된 집합(predefined sets): 각 엔티티 **feature("type")**, 예: MainSig, AxleCounter 등.

```
end = MainSig \union BufferStop
detectionSections = delimitedAreas AxleCounter
routePaths = { (start=s, end=e, path=p) | MainSig s <- Entity,
                                       Path p, Entity e <- directedNeighbors s end }
routes = { *routePath*,
           sections = { a <- detectionSections | intersects a routePath.path }
         | routePath <- routePaths }
```

## Static interlocking editor / 정적 인터로킹 편집기

* 경로 기준(Route criteria)? (또는 Customdata 스크립트)
* Interlocking window:
  - 인식된 interlocking 데이터를 목록으로 표시하고 hover‑to‑show

## Dispatch model / 디스패치 모델

Vehicles / 차량
Dispatch — 시간 지정 이벤트 목록(list of timed events)
현재 선택된 디스패치(타임라인 창 열림)

## Dispatch editor / 디스패치 편집기

* 별도 창/메뉴에서 차량 편집(Edit vehicles)?
* 새로 추가 / history·dispatch·scenario 간 선택
* 현재 활성 디스패치의 **Timeline view**

  * 선택 항목 컨텍스트 메뉴 확장:
    a. border: start train here
    b. signal: train route from here // overlap swing here?
  * 오버드로우 뷰 레이어 확장(with tooltips):
    a. train positions
    b. switch and detection section state
