import os
import json
from pathlib import Path
from typing import Optional, Any
import sys
import os
from enum import IntEnum

from aqt import mw, gui_hooks
from aqt.qt import (
    QMenuBar, QMenu, QAction, QWidget, QHBoxLayout,
    QLabel, QSpinBox, QPushButton, QComboBox, QFont, Qt
)
from aqt.editor import Editor
from dataclasses import dataclass
from anki.notes import Note
# ---------------------------------------------------------------------------
# Pydantic models
# ---------------------------------------------------------------------------
class Kyu(IntEnum):
    Kyu1  = 1;  Jun1 = 2;  Kyu2 = 3;  Jun2 = 4
    Kyu3  = 5;  Kyu4 = 6;  Kyu5 = 7;  Kyu6 = 8
    Kyu7  = 9;  Kyu8 = 10; Kyu9 = 11; Kyu10 = 12

KYU_LABELS = {
    Kyu.Kyu10: "10級", Kyu.Kyu9: "9級",  Kyu.Kyu8: "8級",
    Kyu.Kyu7:  "7級",  Kyu.Kyu6: "6級",  Kyu.Kyu5: "5級",
    Kyu.Kyu4:  "4級",  Kyu.Kyu3: "3級",  Kyu.Jun2: "準2級",
    Kyu.Kyu2:  "2級",  Kyu.Jun1: "準1級", Kyu.Kyu1: "1級",
}
@dataclass
class QuestionItem:
    field_id: int
    question_id: int
    level: Kyu
    year: int
    kind: int
    matter: str
    format: str
    sentence: str
    answer_choices: str
    correct_answer_list: list[str]
    use_word_list: list[str]
    selected_index: int

@dataclass
class FieldInfo:
    field_id: int
    level: Kyu
    name: str
    preamble: str
    count_per_exam: int
@dataclass
class QuestionField:
    field_info: FieldInfo
    items: list[QuestionItem]

# ---------------------------------------------------------------------------
# Data loading
# ---------------------------------------------------------------------------

_DATA: list[QuestionField] = []
def _load_data() -> list[QuestionField]:
    path = Path(__file__).parent / "user_files" / "combined_fields.json"
    with open(path, encoding="utf-8") as f:
        raw = json.load(f)
    return [_parse_field(d) for d in raw]
def _parse_kyu(value: Any) -> Kyu:
    if isinstance(value, int):
        return Kyu(value)
    return Kyu[value] 
def _parse_field(d: dict[str, Any]) -> QuestionField:
    fi = d["field_info"]
    info = FieldInfo(
        field_id=fi["field_id"],
        level=_parse_kyu(fi["level"]),
        name=fi["name"],
        preamble=fi["preamble"],
        count_per_exam=fi["count_per_exam"],
    )
    items = [
        QuestionItem(
            field_id=i["field_id"],
            question_id=i["question_id"],
            level=_parse_kyu(i["level"]),
            year=i["year"],
            kind=i["kind"],
            matter=i["matter"],
            format=i["format"],
            sentence=i["sentence"],
            answer_choices=i["answer_choices"],
            correct_answer_list=i["correct_answer_list"],
            use_word_list=i["use_word_list"],
            selected_index=i["selected_index"],
        )
        for i in d["items"]
    ]
    return QuestionField(field_info=info, items=items)
# ---------------------------------------------------------------------------
# Browser widget
# ---------------------------------------------------------------------------

_instances: dict = {}

class KankenBar(QWidget):
    def __init__(self, editor: Editor):
        super().__init__(editor.widget)
        self.editor = editor
        self._prev_sentence: str = ""
        self._prev_sentence_de: str = ""

        font = QFont()
        font.setPointSize(14)

        layout = QHBoxLayout(self)
        layout.setContentsMargins(4, 2, 4, 2)

        # Kyu selector
        self.kyu_combo = QComboBox()
        self.kyu_combo.setFont(font)
        for kyu in reversed(list(Kyu)):
            self.kyu_combo.addItem(KYU_LABELS[kyu], kyu)
        self.kyu_combo.currentIndexChanged.connect(self._on_kyu_changed)
        layout.addWidget(self.kyu_combo)

        # Field/type selector
        self.field_combo = QComboBox()
        self.field_combo.setFont(font)
        self.field_combo.currentIndexChanged.connect(self._on_field_changed)
        layout.addWidget(self.field_combo)

        # Prev button
        btn_prev = QPushButton("前")
        btn_prev.setFont(font)
        btn_prev.clicked.connect(self._prev)
        layout.addWidget(btn_prev)

        # Question ID spinbox
        self.id_spin = QSpinBox()
        self.id_spin.setFont(font)
        self.id_spin.setMinimum(1)
        self.id_spin.valueChanged.connect(self._on_id_changed)
        layout.addWidget(self.id_spin)

        # Next button
        btn_next = QPushButton("次")
        btn_next.setFont(font)
        btn_next.clicked.connect(self._next)
        layout.addWidget(btn_next)

        # Insert button
        btn_insert = QPushButton("挿入")
        btn_insert.setFont(font)
        btn_insert.clicked.connect(self._insert)
        layout.addWidget(btn_insert)

        # Init state
        self._current_field: Optional[QuestionField] = None
        self._populate_fields()

        editor.outerLayout.insertWidget(0, self)

    # ------------------------------------------------------------------

    def _fields_for_kyu(self, kyu: Kyu) -> list[QuestionField]:
        return [f for f in _DATA if f.field_info.level == kyu]

    def _on_kyu_changed(self) -> None:
        self._populate_fields()

    def _populate_fields(self) -> None:
        kyu = self.kyu_combo.currentData()
        self.field_combo.blockSignals(True)
        self.field_combo.clear()
        for f in self._fields_for_kyu(kyu):
            self.field_combo.addItem(f.field_info.name, f)
        self.field_combo.blockSignals(False)
        self._on_field_changed()

    def _on_field_changed(self) -> None:
        self._current_field = self.field_combo.currentData()
        if self._current_field is None:
            return
        self.id_spin.blockSignals(True)
        self.id_spin.setMaximum(len(self._current_field.items))
        self.id_spin.setValue(1)
        self.id_spin.blockSignals(False)

    def _on_id_changed(self) -> None:
        self._insert()

    def _prev(self) -> None:
        v = self.id_spin.value()
        if v > 1:
            self.id_spin.setValue(v - 1)

    def _next(self) -> None:
        v = self.id_spin.value()
        if v < self.id_spin.maximum():
            self.id_spin.setValue(v + 1)

    def _current_question(self) -> Optional[QuestionItem]:
        if self._current_field is None:
            return None
        idx = self.id_spin.value() - 1
        return self._current_field.items[idx]

    def _render_sentence(self, q: QuestionItem) -> str:
        result = ""
        remaining = q.sentence
        bracket_index = 0

        while "｛" in remaining:
            open_idx = remaining.find("｛")
            result += remaining[:open_idx]
            remaining = remaining[open_idx + 1:]
            close_idx = remaining.find("｝")
            if close_idx == -1:
                break
            word = remaining[:close_idx]
            if bracket_index == q.selected_index:
                result += f'<span style="color: red;">{word}</span>'
            else:
                result += word
            remaining = remaining[close_idx + 1:]
            bracket_index += 1

        result += remaining
        return result
    def _insert(self) -> None:
        q = self._current_question()
        if q is None:
            return
        if self._current_field is None:
            return
        unique_id = f"kanken_{q.level.value}_{q.field_id}_{q.question_id}"
        info = self._current_field.field_info
       

        self._set_field("sentence",self._render_sentence(q))
        self._set_field("answer", "、".join(q.correct_answer_list))
        self._set_field("kyu", KYU_LABELS[info.level])
        self._set_field("field_name", info.name)
        self._set_field("year", str(q.year))
        self._set_field("question_id", unique_id)
        self._set_field("used_word", "、".join(q.use_word_list))
        self._set_field("dict","、".join(q.use_word_list))
        
        if q.sentence == self._prev_sentence and self._prev_sentence_de:
            self._set_field("sentence_de", self._prev_sentence_de)

    def _set_field(self, name: str, value: str) -> None:
        note = self.editor.note
        if note is None:
            return
        tp = note.note_type()
        if tp is None:
            return
        for idx, field in enumerate(tp["flds"]):
            if field["name"] == name:
                note.fields[idx] = value
                self.editor.set_note(note)
                self.editor.loadNote()
                return

# ---------------------------------------------------------------------------
# Hook
# ---------------------------------------------------------------------------

def _create(editor: Editor) -> None:
    _instances[editor] = KankenBar(editor)

_DATA = _load_data()
gui_hooks.editor_did_init.append(_create)
def _on_note_added(note: Note) -> None:
    dead = []
    for editor, instance in _instances.items():
        try:
            q = instance._current_question()
            if q is not None:
                instance._prev_sentence = q.sentence
            tp = note.note_type()
            if tp is not None:
                for idx, field in enumerate(tp["flds"]):
                    if field["name"] == "sentence_de":
                        instance._prev_sentence_de = note.fields[idx]
                        break
            instance._next()
        except RuntimeError:
            dead.append(editor)
    for editor in dead:
        del _instances[editor]
gui_hooks.add_cards_did_add_note.append(_on_note_added)