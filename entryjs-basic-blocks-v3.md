# EntryJS 블럭 구현 보고서

## 1. 개요

- 저장소: [entrylabs/entryjs](https://github.com/entrylabs/entryjs)
- 클론: 2026-08-08 (기본 브랜치 `develop`)
- 블럭 정의 위치: `src/playground/blocks/`
- 블럭 이름 출처: `extern/lang/ko.js`의 `Lang.template` (한글, □는 입력 슬롯)
- **구현 대상 블럭 수(고유 ID): 기본 203 + 인공지능 105 + 확장 42 = **350**개**

## 2. 코드 위치 (GitHub 링크)

| 그룹 | 블럭 파일 |
|---|---|
| (전체) | [src/playground/blocks](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks) |
| 시작 | [block_start.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_start.js) |
| 흐름 | [block_flow.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_flow.js) |
| 움직임 | [block_moving.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_moving.js) |
| 형태 | [block_looks.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_looks.js) |
| 붓 | [block_brush.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_brush.js) |
| 텍스트 | [block_text.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_text.js) |
| 소리 | [block_sound.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_sound.js) |
| 판단 | [block_judgement.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_judgement.js) |
| 연산 | [block_calc.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_calc.js) |
| 변수 | [block_variable.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_variable.js) |
| 함수 | [block_func.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_func.js) |
| 데이터분석 | [block_analysis.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_analysis.js) |
| 인공지능 (전체) | [block_ai_*.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks) |
| 인공지능 | [block_ai_learning.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_ai_learning.js) |
| 인공지능 | [block_ai_learning_cluster.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_ai_learning_cluster.js) |
| 인공지능 | [block_ai_learning_decisiontree.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_ai_learning_decisiontree.js) |
| 인공지능 | [block_ai_learning_knn.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_ai_learning_knn.js) |
| 인공지능 | [block_ai_learning_logistic_regression.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_ai_learning_logistic_regression.js) |
| 인공지능 | [block_ai_learning_regression.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_ai_learning_regression.js) |
| 인공지능 | [block_ai_learning_svm.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_ai_learning_svm.js) |
| 인공지능 | [block_ai_utilize_audio.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_ai_utilize_audio.js) |
| 인공지능 | [block_ai_utilize_face_landmarker.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_ai_utilize_face_landmarker.js) |
| 인공지능 | [block_ai_utilize_gesture_recognition.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_ai_utilize_gesture_recognition.js) |
| 인공지능 | [block_ai_utilize_media_pipe.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_ai_utilize_media_pipe.js) |
| 인공지능 | [block_ai_utilize_object_detector.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_ai_utilize_object_detector.js) |
| 인공지능 | [block_ai_utilize_pose_landmarker.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_ai_utilize_pose_landmarker.js) |
| 인공지능 | [block_ai_utilize_translate.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_ai_utilize_translate.js) |
| 인공지능 | [block_ai_utilize_tts.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_ai_utilize_tts.js) |
| 인공지능 | [block_ai_utilize_video.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_ai_utilize_video.js) |
| 확장 (전체) | [block_expansion_*.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks) |
| 확장 | [block_expansion_behaviorconduct_disaster.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_expansion_behaviorconduct_disaster.js) |
| 확장 | [block_expansion_behaviorconduct_lifesafety.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_expansion_behaviorconduct_lifesafety.js) |
| 확장 | [block_expansion_disasterAlert.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_expansion_disasterAlert.js) |
| 확장 | [block_expansion_emergencyActionGuidelines.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_expansion_emergencyActionGuidelines.js) |
| 확장 | [block_expansion_festival.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_expansion_festival.js) |
| 확장 | [block_expansion_weather.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/block_expansion_weather.js) |
| (블럭 등록/집계) | [index.js](https://github.com/entrylabs/entryjs/blob/develop/src/playground/blocks/index.js) |
| (한글 이름) | [extern/lang/ko.js](https://github.com/entrylabs/entryjs/blob/develop/extern/lang/ko.js) |
| (하드웨어 블럭) | [hardware/](https://github.com/entrylabs/entryjs/tree/develop/src/playground/blocks/hardware), [hardwareLite/](https://github.com/entrylabs/entryjs/tree/develop/src/playground/blocks/hardwareLite) |

## 3. 기본 블럭 (203개)

> 데이터 분석(`block_analysis.js`, 18개)도 여기 포함.

### 시작 (26)

| 블럭 ID | 블럭 이름 |
|---|---|
| messageAddButton | □ |
| when_run_button_click | □ 시작하기 버튼을 클릭했을 때 |
| when_some_key_pressed | □ □ 키를 눌렀을 때 |
| mouse_clicked | □ 마우스를 클릭했을 때 |
| mouse_click_cancled | □ 마우스 클릭을 해제했을 때 |
| when_object_click | □ 오브젝트를 클릭했을 때 |
| when_object_click_canceled | □ 오브젝트 클릭을 해제했을 때 |
| when_message_cast | □ □ 신호를 받았을 때 |
| message_cast | □ 신호 보내기 □ |
| message_cast_wait | □ 신호 보내고 기다리기 □ |
| when_scene_start | □ 장면이 시작되었을 때 |
| start_scene | □ 시작하기 □ |
| start_neighbor_scene | □ 장면 시작하기 □ |
| check_object_property | (이름 없음) |
| check_block_execution | (이름 없음) |
| switch_scope | (이름 없음) |
| is_answer_submited | (이름 없음) |
| check_lecture_goal | (이름 없음) |
| check_variable_by_name | (이름 없음) |
| show_prompt | (이름 없음) |
| check_goal_success | (이름 없음) |
| positive_number | (이름 없음) |
| negative_number | (이름 없음) |
| wildcard_string | (이름 없음) |
| wildcard_boolean | (이름 없음) |
| register_score | (이름 없음) |

### 흐름 (15)

| 블럭 ID | 블럭 이름 |
|---|---|
| wait_second | □ 초 기다리기 □ |
| repeat_basic | □ 번 반복하기 □ |
| repeat_inf | 계속 반복하기 □ |
| repeat_while_true | □ □ 반복하기 □ |
| stop_repeat | 반복 중단하기 □ |
| continue_repeat | 이번 반복 건너뛰기 □ |
| _if | 만일 □ (이)라면 □ |
| if_else | 만일 □ (이)라면 □ □ 아니면 |
| wait_until_true | □ 이(가) 될 때까지 기다리기 □ |
| stop_object | □ 코드 멈추기 □ |
| restart_project | 처음부터 다시 실행하기 □ |
| when_clone_start | □ 복제본이 처음 생성되었을 때 |
| create_clone | □ 의 복제본 만들기 □ |
| delete_clone | 이 복제본 삭제하기 □ |
| remove_all_clones | 모든 복제본 삭제하기 □ |

### 움직임 (19)

| 블럭 ID | 블럭 이름 |
|---|---|
| move_direction | 이동 방향으로 □ 만큼 움직이기 □ |
| bounce_wall | 화면 끝에 닿으면 튕기기 □ |
| move_x | x 좌표를 □ 만큼 바꾸기 □ |
| move_y | y 좌표를 □ 만큼 바꾸기 □ |
| move_xy_time | □ 초 동안 x: □ y: □ 만큼 움직이기 □ |
| locate_x | x: □ 위치로 이동하기 □ |
| locate_y | y: □ 위치로 이동하기 □ |
| locate_xy | x: □ y: □ 위치로 이동하기 □ |
| locate_xy_time | □ 초 동안 x: □ y: □ 위치로 이동하기 □ |
| locate | □ 위치로 이동하기 □ |
| locate_object_time | □ 초 동안 □ 위치로 이동하기 □ |
| rotate_relative | 방향을 □ 만큼 회전하기 □ |
| direction_relative | 이동 방향을 □ 만큼 회전하기 □ |
| rotate_by_time | □ 초 동안 방향을 □ 만큼 회전하기 □ |
| direction_relative_duration | □ 초 동안 이동 방향 □ 만큼 회전하기 □ |
| rotate_absolute | 방향을 □ (으)로 정하기 □ |
| direction_absolute | 이동 방향을 □ (으)로 정하기 □ |
| see_angle_object | □ 쪽 바라보기 □ |
| move_to_angle | □ 방향으로 □ 만큼 움직이기 □ |

### 형태 (17)

| 블럭 ID | 블럭 이름 |
|---|---|
| show | 모양 보이기 □ |
| hide | 모양 숨기기 □ |
| dialog_time | □ 을(를) □ 초 동안 □ □ |
| dialog | □ 을(를) □ □ |
| remove_dialog | 말풍선 지우기 □ |
| change_to_some_shape | □ 모양으로 바꾸기 □ |
| change_to_next_shape | □ 모양으로 바꾸기 □ |
| add_effect_amount | □ 효과를 □ 만큼 주기 □ |
| change_effect_amount | □ 효과를 □ (으)로 정하기 □ |
| erase_all_effects | 효과 모두 지우기 □ |
| change_scale_size | 크기를 □ 만큼 바꾸기 □ |
| set_scale_size | 크기를 □ (으)로 정하기 □ |
| stretch_scale_size | □ 를 □ 만큼 늘이기 □ |
| reset_scale_size | 원래 크기로 되돌리기 □ |
| flip_x | 상하 모양 뒤집기 □ |
| flip_y | 좌우 모양 뒤집기 □ |
| change_object_index | □ 보내기 □ |

### 붓 (13)

| 블럭 ID | 블럭 이름 |
|---|---|
| brush_stamp | 도장 찍기 □ |
| start_drawing | 그리기 시작하기 □ |
| stop_drawing | 그리기 멈추기 □ |
| start_fill | 채우기 시작하기 □ |
| stop_fill | 채우기 멈추기 □ |
| set_color | 그리기 색을 □ (으)로 정하기 □ |
| set_random_color | 붓의 색을 무작위로 정하기 □ |
| set_fill_color | 채우기 색을 □ (으)로 정하기 □ |
| change_thickness | 그리기 굵기를 □ 만큼 바꾸기 □ |
| set_thickness | 그리기 굵기를 □ (으)로 정하기 □ |
| change_brush_transparency | 붓의 투명도를 □ % 만큼 바꾸기 □ |
| set_brush_tranparency | 붓의 투명도를 □ % 로 정하기 □ |
| brush_erase_all | 모든 붓 지우기 □ |

### 텍스트 (9)

| 블럭 ID | 블럭 이름 |
|---|---|
| text_read | 글상자 □의 내용 |
| text_write | □ (이)라고 글쓰기 □ |
| text_append | □ 을(를) 뒤에 추가하기 □ |
| text_prepend | □ 을(를) 앞에 추가하기 □ |
| text_change_effect | 텍스트에 □ 효과 □ □ |
| text_change_font | 글씨체를 □ (으)로 바꾸기 □ |
| text_change_font_color | 글씨색을 □ (으)로 바꾸기 □ |
| text_change_bg_color | 배경색을 □ (으)로 바꾸기 □ |
| text_flush | 텍스트 모두 지우기 □ |

### 소리 (16)

| 블럭 ID | 블럭 이름 |
|---|---|
| sound_something_with_block | 소리 □ 재생하기 □ |
| sound_something_second_with_block | 소리 □ □ 초 재생하기 □ |
| sound_from_to | 소리 □ □ 초 부터 □ 초까지 재생하기 □ |
| sound_something_wait_with_block | 소리 □ 재생하고 기다리기 □ |
| sound_something_second_wait_with_block | 소리 □ □ 초 재생하고 기다리기 □ |
| sound_from_to_and_wait | 소리 □ □ 초 부터 □ 초까지 재생하고 기다리기 □ |
| sound_volume_change | 소리 크기를 □ 만큼 바꾸기 □ |
| sound_volume_set | 소리 크기를 □ % 로 정하기 □ |
| get_sound_speed | 소리 빠르기 |
| sound_speed_change | 소리 빠르기를 □ 만큼 바꾸기 □ |
| sound_speed_set | 소리 빠르기를 □ 배로 정하기 □ |
| sound_silent_all | □ 소리 멈추기 □ |
| play_bgm | □ 을(를) 배경음악으로 재생하기 □ |
| stop_bgm | 배경음악 멈추기 □ |
| get_sound_volume | □ □ |
| get_sound_duration | □ □ □ |

### 판단 (11)

| 블럭 ID | 블럭 이름 |
|---|---|
| is_clicked | □ |
| is_object_clicked | □ |
| is_press_some_key | □ □ |
| reach_something | □ □ □ |
| is_type | □ □ □ □ |
| boolean_basic_operator | □ □ □ |
| boolean_and_or | □ □ □ |
| boolean_not | □ □ □ |
| is_boost_mode | □ |
| is_current_device_type | □ 에서 실행하는가? |
| is_touch_supported | 화면을 터치할 수 있는가? |

### 연산 (26)

| 블럭 ID | 블럭 이름 |
|---|---|
| calc_basic | □ □ □ |
| calc_rand | □ □ □ □ □ |
| coordinate_mouse | □ □ □ |
| coordinate_object | □ □ □ □ |
| quotient_and_mod | □ □ □ □ □ □ |
| calc_operation | □ □ □ □ |
| get_project_timer_value | □ □ |
| choose_project_timer_action | □ □ □ □ |
| set_visible_project_timer | □ □ □ □ |
| get_date | □ □ □ |
| distance_something | □ □ □ |
| get_user_name | 아이디 |
| get_nickname | 닉네임 |
| length_of_string | □ □ □ |
| reverse_of_string | □ □ □ |
| combine_something | □ □ □ □ □ |
| char_at | □ □ □ □ □ |
| substring | □ □ □ □ □ □ □ |
| count_match_string | □ □ □ □ |
| index_of_string | □ □ □ □ □ |
| replace_string | □ □ □ □ □ □ □ |
| change_string_case | □ □ □ □ |
| get_block_count | □ 의 블록 수 |
| change_rgb_to_hex | R:□G:□B:□의 HEX 값 |
| change_hex_to_rgb | HEX□의 □값 |
| get_boolean_value | □ 의 값 |

### 변수 (19)

| 블럭 ID | 블럭 이름 |
|---|---|
| variableAddButton | □ |
| listAddButton | □ |
| ask_and_wait | □ 을(를) 묻고 대답 기다리기 □ |
| get_canvas_input_value | □ |
| set_visible_answer | 대답 □ □ |
| get_variable | □ □ |
| change_variable | □ 에 □ 만큼 더하기 □ |
| set_variable | □ 를 □ (으)로 정하기 □ |
| show_variable | 변수 □ 보이기 □ |
| hide_variable | 변수 □ 숨기기 □ |
| value_of_index_from_list | □ □ □ □ □ |
| add_value_to_list | □ 항목을 □ 에 추가하기 □ |
| remove_value_from_list | □ 번째 항목을 □ 에서 삭제하기 □ |
| insert_value_to_list | □ 을(를) □ 의 □ 번째에 넣기 □ |
| change_value_list_index | □ □ 번째 항목을 □ (으)로 바꾸기 □ |
| length_of_list | □ □ □ |
| is_included_in_list | □ □ □ □ □ |
| show_list | 리스트 □ 보이기 □ |
| hide_list | 리스트 □ 숨기기 □ |

### 함수 (14)

| 블럭 ID | 블럭 이름 |
|---|---|
| functionAddButton | □ |
| function_name | (이름 없음) |
| showFunctionPropsButton | (이름 없음) |
| set_func_variable | (이름 없음) |
| get_func_variable | (이름 없음) |
| function_create_value | 함수 정의하기 □ □ □ 결괏값을 □ (으)로 정하기 |
| function_general | 함수 □ |
| function_value | 함수 |
| function_field_label | □□ |
| function_field_string | □□ |
| function_field_boolean | □□ |
| function_param_string | 문자/숫자값 |
| function_param_boolean | 판단값 |
| function_create | 함수 정의하기 □ □ |

### 데이터분석 (18)

| 블럭 ID | 블럭 이름 |
|---|---|
| analizyDataAddButton | □ |
| append_row_to_table | 테이블 □에 □ 추가하기 □ |
| insert_row_to_table | 테이블 □ □ 번째에 □ 추가하기 □ |
| delete_row_from_table | 테이블 □ □번째 □ 삭제하기 □ |
| set_value_from_table | 테이블 □ □번째 행의 □을(를) □(으)로 바꾸기 □ |
| save_current_table | 테이블 □ 을(를) 현재 상태로 남기기 □ |
| get_table_count | 테이블 □의 □ 개수 |
| get_value_from_table | 테이블 □ □번째 행의 □ 값 |
| get_value_from_last_row | 테이블 □ 마지막 행의 □ 값 |
| calc_values_from_table | 테이블 □ □의 □ |
| open_table | 테이블 □ 창 열기 □ |
| open_table_wait | 테이블 □ 창을 □ 초 동안 열기 □ |
| open_table_chart | 테이블 □ 의 □ 차트 창 열기 □ |
| close_table_chart | 테이블 차트 창 닫기 □ |
| get_coefficient | 테이블 □ □ 과(와) □ 의 상관계수 |
| set_value_from_cell | 테이블 □ 의 □ 셀 값을 □ (으)로 바꾸기 □ |
| get_value_from_cell | 테이블 □ 의 □ 셀 값 |
| get_value_v_lookup | 테이블 □ 의 □ 이(가) □ 인 행의 □ 값 |

## 4. 인공지능(AI) 블럭 (105개 고유)

> 학습(learning) 하위 파일들은 공유 학습 블럭을 재사용하므로 고유 블럭 수는 파일 수보다 적음.

### block_ai_learning.js (26개, 그 중 고유 26개) 

| 블럭 ID | 블럭 이름 |
|---|---|
| get_predict_1 | □ □ 의 분류 결과 |
| get_predict_2 | □ □ □ □ 의 분류 결과 |
| get_predict_3 | □ □ □ □ □ □ 의 분류 결과 |
| get_predict_4 | □ □ □ □ □ □ □ □ 의 분류 결과 |
| get_predict_5 | □ □ □ □ □ □ □ □ □ □ 의 분류 결과 |
| get_predict_6 | □ □ □ □ □ □ □ □ □ □ □ □ 의 분류 결과 |
| is_result_1 | □ □ 의 분류 결과가 □ 인가? |
| is_result_2 | □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_3 | □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_4 | □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_5 | □ □ □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_6 | □ □ □ □ □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| learning_title_image | □ |
| learning_title_speech | □ |
| learning_title_text | □ |
| insert_data_for_test | 학습한 모델로 분류하기 □ |
| video_capture_for_image_test | 비디오 화면을 학습한 모델로 분류 □ □ |
| insert_text_block_for_test | □ 을(를) 학습한 모델로 분류하기 □ |
| test_result | 분류 결과 |
| accuracy_of_result | □에 대한 신뢰도 |
| is_group | 분류 결과가 □ 인가? |
| retrain_model | 모델 다시 학습하기 □ |
| model_is_trained | 모델이 학습되었는가? |
| set_train_visible | 모델 □ □ |
| set_train_chart | 모델 차트 창 □ □ |
| get_result_info | 모델의 □ |

### block_ai_learning_cluster.js (26개, 그 중 고유 26개) 

| 블럭 ID | 블럭 이름 |
|---|---|
| get_predict_1 | □ □ 의 분류 결과 |
| get_predict_2 | □ □ □ □ 의 분류 결과 |
| get_predict_3 | □ □ □ □ □ □ 의 분류 결과 |
| get_predict_4 | □ □ □ □ □ □ □ □ 의 분류 결과 |
| get_predict_5 | □ □ □ □ □ □ □ □ □ □ 의 분류 결과 |
| get_predict_6 | □ □ □ □ □ □ □ □ □ □ □ □ 의 분류 결과 |
| is_result_1 | □ □ 의 분류 결과가 □ 인가? |
| is_result_2 | □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_3 | □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_4 | □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_5 | □ □ □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_6 | □ □ □ □ □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| learning_title_image | □ |
| learning_title_speech | □ |
| learning_title_text | □ |
| insert_data_for_test | 학습한 모델로 분류하기 □ |
| video_capture_for_image_test | 비디오 화면을 학습한 모델로 분류 □ □ |
| insert_text_block_for_test | □ 을(를) 학습한 모델로 분류하기 □ |
| test_result | 분류 결과 |
| accuracy_of_result | □에 대한 신뢰도 |
| is_group | 분류 결과가 □ 인가? |
| retrain_model | 모델 다시 학습하기 □ |
| model_is_trained | 모델이 학습되었는가? |
| set_train_visible | 모델 □ □ |
| set_train_chart | 모델 차트 창 □ □ |
| get_result_info | 모델의 □ |

### block_ai_learning_decisiontree.js (26개, 그 중 고유 26개) 

| 블럭 ID | 블럭 이름 |
|---|---|
| get_predict_1 | □ □ 의 분류 결과 |
| get_predict_2 | □ □ □ □ 의 분류 결과 |
| get_predict_3 | □ □ □ □ □ □ 의 분류 결과 |
| get_predict_4 | □ □ □ □ □ □ □ □ 의 분류 결과 |
| get_predict_5 | □ □ □ □ □ □ □ □ □ □ 의 분류 결과 |
| get_predict_6 | □ □ □ □ □ □ □ □ □ □ □ □ 의 분류 결과 |
| is_result_1 | □ □ 의 분류 결과가 □ 인가? |
| is_result_2 | □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_3 | □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_4 | □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_5 | □ □ □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_6 | □ □ □ □ □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| learning_title_image | □ |
| learning_title_speech | □ |
| learning_title_text | □ |
| insert_data_for_test | 학습한 모델로 분류하기 □ |
| video_capture_for_image_test | 비디오 화면을 학습한 모델로 분류 □ □ |
| insert_text_block_for_test | □ 을(를) 학습한 모델로 분류하기 □ |
| test_result | 분류 결과 |
| accuracy_of_result | □에 대한 신뢰도 |
| is_group | 분류 결과가 □ 인가? |
| retrain_model | 모델 다시 학습하기 □ |
| model_is_trained | 모델이 학습되었는가? |
| set_train_visible | 모델 □ □ |
| set_train_chart | 모델 차트 창 □ □ |
| get_result_info | 모델의 □ |

### block_ai_learning_knn.js (26개, 그 중 고유 26개) 

| 블럭 ID | 블럭 이름 |
|---|---|
| get_predict_1 | □ □ 의 분류 결과 |
| get_predict_2 | □ □ □ □ 의 분류 결과 |
| get_predict_3 | □ □ □ □ □ □ 의 분류 결과 |
| get_predict_4 | □ □ □ □ □ □ □ □ 의 분류 결과 |
| get_predict_5 | □ □ □ □ □ □ □ □ □ □ 의 분류 결과 |
| get_predict_6 | □ □ □ □ □ □ □ □ □ □ □ □ 의 분류 결과 |
| is_result_1 | □ □ 의 분류 결과가 □ 인가? |
| is_result_2 | □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_3 | □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_4 | □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_5 | □ □ □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_6 | □ □ □ □ □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| learning_title_image | □ |
| learning_title_speech | □ |
| learning_title_text | □ |
| insert_data_for_test | 학습한 모델로 분류하기 □ |
| video_capture_for_image_test | 비디오 화면을 학습한 모델로 분류 □ □ |
| insert_text_block_for_test | □ 을(를) 학습한 모델로 분류하기 □ |
| test_result | 분류 결과 |
| accuracy_of_result | □에 대한 신뢰도 |
| is_group | 분류 결과가 □ 인가? |
| retrain_model | 모델 다시 학습하기 □ |
| model_is_trained | 모델이 학습되었는가? |
| set_train_visible | 모델 □ □ |
| set_train_chart | 모델 차트 창 □ □ |
| get_result_info | 모델의 □ |

### block_ai_learning_logistic_regression.js (26개, 그 중 고유 26개) 

| 블럭 ID | 블럭 이름 |
|---|---|
| get_predict_1 | □ □ 의 분류 결과 |
| get_predict_2 | □ □ □ □ 의 분류 결과 |
| get_predict_3 | □ □ □ □ □ □ 의 분류 결과 |
| get_predict_4 | □ □ □ □ □ □ □ □ 의 분류 결과 |
| get_predict_5 | □ □ □ □ □ □ □ □ □ □ 의 분류 결과 |
| get_predict_6 | □ □ □ □ □ □ □ □ □ □ □ □ 의 분류 결과 |
| is_result_1 | □ □ 의 분류 결과가 □ 인가? |
| is_result_2 | □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_3 | □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_4 | □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_5 | □ □ □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_6 | □ □ □ □ □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| learning_title_image | □ |
| learning_title_speech | □ |
| learning_title_text | □ |
| insert_data_for_test | 학습한 모델로 분류하기 □ |
| video_capture_for_image_test | 비디오 화면을 학습한 모델로 분류 □ □ |
| insert_text_block_for_test | □ 을(를) 학습한 모델로 분류하기 □ |
| test_result | 분류 결과 |
| accuracy_of_result | □에 대한 신뢰도 |
| is_group | 분류 결과가 □ 인가? |
| retrain_model | 모델 다시 학습하기 □ |
| model_is_trained | 모델이 학습되었는가? |
| set_train_visible | 모델 □ □ |
| set_train_chart | 모델 차트 창 □ □ |
| get_result_info | 모델의 □ |

### block_ai_learning_regression.js (26개, 그 중 고유 26개) 

| 블럭 ID | 블럭 이름 |
|---|---|
| get_predict_1 | □ □ 의 분류 결과 |
| get_predict_2 | □ □ □ □ 의 분류 결과 |
| get_predict_3 | □ □ □ □ □ □ 의 분류 결과 |
| get_predict_4 | □ □ □ □ □ □ □ □ 의 분류 결과 |
| get_predict_5 | □ □ □ □ □ □ □ □ □ □ 의 분류 결과 |
| get_predict_6 | □ □ □ □ □ □ □ □ □ □ □ □ 의 분류 결과 |
| is_result_1 | □ □ 의 분류 결과가 □ 인가? |
| is_result_2 | □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_3 | □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_4 | □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_5 | □ □ □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_6 | □ □ □ □ □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| learning_title_image | □ |
| learning_title_speech | □ |
| learning_title_text | □ |
| insert_data_for_test | 학습한 모델로 분류하기 □ |
| video_capture_for_image_test | 비디오 화면을 학습한 모델로 분류 □ □ |
| insert_text_block_for_test | □ 을(를) 학습한 모델로 분류하기 □ |
| test_result | 분류 결과 |
| accuracy_of_result | □에 대한 신뢰도 |
| is_group | 분류 결과가 □ 인가? |
| retrain_model | 모델 다시 학습하기 □ |
| model_is_trained | 모델이 학습되었는가? |
| set_train_visible | 모델 □ □ |
| set_train_chart | 모델 차트 창 □ □ |
| get_result_info | 모델의 □ |

### block_ai_learning_svm.js (26개, 그 중 고유 26개) 

| 블럭 ID | 블럭 이름 |
|---|---|
| get_predict_1 | □ □ 의 분류 결과 |
| get_predict_2 | □ □ □ □ 의 분류 결과 |
| get_predict_3 | □ □ □ □ □ □ 의 분류 결과 |
| get_predict_4 | □ □ □ □ □ □ □ □ 의 분류 결과 |
| get_predict_5 | □ □ □ □ □ □ □ □ □ □ 의 분류 결과 |
| get_predict_6 | □ □ □ □ □ □ □ □ □ □ □ □ 의 분류 결과 |
| is_result_1 | □ □ 의 분류 결과가 □ 인가? |
| is_result_2 | □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_3 | □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_4 | □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_5 | □ □ □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| is_result_6 | □ □ □ □ □ □ □ □ □ □ □ □ 의 분류 결과가 □ 인가? |
| learning_title_image | □ |
| learning_title_speech | □ |
| learning_title_text | □ |
| insert_data_for_test | 학습한 모델로 분류하기 □ |
| video_capture_for_image_test | 비디오 화면을 학습한 모델로 분류 □ □ |
| insert_text_block_for_test | □ 을(를) 학습한 모델로 분류하기 □ |
| test_result | 분류 결과 |
| accuracy_of_result | □에 대한 신뢰도 |
| is_group | 분류 결과가 □ 인가? |
| retrain_model | 모델 다시 학습하기 □ |
| model_is_trained | 모델이 학습되었는가? |
| set_train_visible | 모델 □ □ |
| set_train_chart | 모델 차트 창 □ □ |
| get_result_info | 모델의 □ |

### block_ai_utilize_audio.js (8개, 그 중 고유 8개) 

| 블럭 ID | 블럭 이름 |
|---|---|
| audio_title | □ |
| check_microphone | 마이크가 연결되었는가? |
| get_microphone_volume | 마이크 소리 크기 |
| speech_to_text_title | □ |
| speech_to_text_convert | □ 음성 인식하기 □ |
| timed_speech_to_text_convert | □ 초 동안 □ 음성 인식하기 □ |
| set_visible_speech_to_text | 인식한 음성 □ □ |
| speech_to_text_get_value | 음성을 문자로 바꾼 값 |

### block_ai_utilize_face_landmarker.js (21개, 그 중 고유 21개) 

| 블럭 ID | 블럭 이름 |
|---|---|
| audio_title | □ |
| check_microphone | 마이크가 연결되었는가? |
| get_microphone_volume | 마이크 소리 크기 |
| speech_to_text_title | □ |
| speech_to_text_convert | □ 음성 인식하기 □ |
| timed_speech_to_text_convert | □ 초 동안 □ 음성 인식하기 □ |
| set_visible_speech_to_text | 인식한 음성 □ □ |
| speech_to_text_get_value | 음성을 문자로 바꾼 값 |
| face_landmarker_title | □ |
| when_face_landmarker | □ 얼굴을 인식했을 때 |
| face_landmarker | 얼굴 인식 □ □ |
| draw_detected_face | 인식한 얼굴 □ □ |
| check_detected_face | 얼굴을 인식했는가? |
| count_detected_face | 인식한 얼굴의 수 |
| locate_to_face | □ 번째의 얼굴의 □ (으)로 이동하기 □ |
| locate_time_to_face | □ 초 동안 □ 번째의 얼굴의 □ (으)로 이동하기 □ |
| check_detected_gender | □ 번째 얼굴의 성별이 □ 인가? |
| check_compare_age | □ 번째 얼굴의 나이 □ □ 인가? |
| check_detected_emotion | □ 번째 얼굴의 감정이 □ 인가? |
| axis_detected_face | □ 번째 얼굴의 □ 의 □ 좌표 |
| get_detected_face_value | □ 번째 얼굴의 □ |

### block_ai_utilize_gesture_recognition.js (34개, 그 중 고유 34개) 

| 블럭 ID | 블럭 이름 |
|---|---|
| audio_title | □ |
| check_microphone | 마이크가 연결되었는가? |
| get_microphone_volume | 마이크 소리 크기 |
| speech_to_text_title | □ |
| speech_to_text_convert | □ 음성 인식하기 □ |
| timed_speech_to_text_convert | □ 초 동안 □ 음성 인식하기 □ |
| set_visible_speech_to_text | 인식한 음성 □ □ |
| speech_to_text_get_value | 음성을 문자로 바꾼 값 |
| face_landmarker_title | □ |
| when_face_landmarker | □ 얼굴을 인식했을 때 |
| face_landmarker | 얼굴 인식 □ □ |
| draw_detected_face | 인식한 얼굴 □ □ |
| check_detected_face | 얼굴을 인식했는가? |
| count_detected_face | 인식한 얼굴의 수 |
| locate_to_face | □ 번째의 얼굴의 □ (으)로 이동하기 □ |
| locate_time_to_face | □ 초 동안 □ 번째의 얼굴의 □ (으)로 이동하기 □ |
| check_detected_gender | □ 번째 얼굴의 성별이 □ 인가? |
| check_compare_age | □ 번째 얼굴의 나이 □ □ 인가? |
| check_detected_emotion | □ 번째 얼굴의 감정이 □ 인가? |
| axis_detected_face | □ 번째 얼굴의 □ 의 □ 좌표 |
| get_detected_face_value | □ 번째 얼굴의 □ |
| hand_detection_title | □ |
| when_hand_detection | □ 손을 인식했을 때 |
| hand_detection | 손 인식 □ □ |
| draw_detected_hand | 인식한 손 □ □ |
| check_detected_hand | 손을 인식했는가? |
| count_detected_hand | 인식한 손의 수 |
| locate_to_hand | □ 번째 손의 □ □ (으)로 이동하기 □ |
| locate_time_to_hand | □ 초 동안 □ 번째 손의 □ □ (으)로 이동하기 □ |
| axis_detected_hand | □ 번째 손의 □ □ 의 □ 좌표 |
| is_which_hand | □ 번째 손이 □ 인가? |
| get_which_hand | □ 번째 손 |
| is_which_gesture | □ 번째 손의 모양이 □ 인가? |
| get_which_gesture | □ 번째 손의 모양 |

### block_ai_utilize_media_pipe.js (7개, 그 중 고유 7개) 

| 블럭 ID | 블럭 이름 |
|---|---|
| media_pipe_title | □ |
| media_pipe_video_screen | 비디오 화면 □ □ |
| media_pipe_switch_camera | □ 카메라로 바꾸기 □ |
| check_connected_camera | 카메라가 연결되었는가? |
| media_pipe_flip_camera | 비디오 화면 □ 뒤집기 □ |
| media_pipe_set_opacity_camera | 비디오 투명도 효과를 □ 으로 정하기 □ |
| media_pipe_motion_value | □ 에서 감지한 □ 값 |

### block_ai_utilize_object_detector.js (41개, 그 중 고유 41개) 

| 블럭 ID | 블럭 이름 |
|---|---|
| audio_title | □ |
| check_microphone | 마이크가 연결되었는가? |
| get_microphone_volume | 마이크 소리 크기 |
| speech_to_text_title | □ |
| speech_to_text_convert | □ 음성 인식하기 □ |
| timed_speech_to_text_convert | □ 초 동안 □ 음성 인식하기 □ |
| set_visible_speech_to_text | 인식한 음성 □ □ |
| speech_to_text_get_value | 음성을 문자로 바꾼 값 |
| face_landmarker_title | □ |
| when_face_landmarker | □ 얼굴을 인식했을 때 |
| face_landmarker | 얼굴 인식 □ □ |
| draw_detected_face | 인식한 얼굴 □ □ |
| check_detected_face | 얼굴을 인식했는가? |
| count_detected_face | 인식한 얼굴의 수 |
| locate_to_face | □ 번째의 얼굴의 □ (으)로 이동하기 □ |
| locate_time_to_face | □ 초 동안 □ 번째의 얼굴의 □ (으)로 이동하기 □ |
| check_detected_gender | □ 번째 얼굴의 성별이 □ 인가? |
| check_compare_age | □ 번째 얼굴의 나이 □ □ 인가? |
| check_detected_emotion | □ 번째 얼굴의 감정이 □ 인가? |
| axis_detected_face | □ 번째 얼굴의 □ 의 □ 좌표 |
| get_detected_face_value | □ 번째 얼굴의 □ |
| hand_detection_title | □ |
| when_hand_detection | □ 손을 인식했을 때 |
| hand_detection | 손 인식 □ □ |
| draw_detected_hand | 인식한 손 □ □ |
| check_detected_hand | 손을 인식했는가? |
| count_detected_hand | 인식한 손의 수 |
| locate_to_hand | □ 번째 손의 □ □ (으)로 이동하기 □ |
| locate_time_to_hand | □ 초 동안 □ 번째 손의 □ □ (으)로 이동하기 □ |
| axis_detected_hand | □ 번째 손의 □ □ 의 □ 좌표 |
| is_which_hand | □ 번째 손이 □ 인가? |
| get_which_hand | □ 번째 손 |
| is_which_gesture | □ 번째 손의 모양이 □ 인가? |
| get_which_gesture | □ 번째 손의 모양 |
| object_detector_title | □ |
| when_object_detector | □ 사물을 인식했을 때 |
| object_detector | 사물 인식 □ □ |
| draw_detected_object | 인식한 사물 □ □ |
| check_detected_object | 사물을 인식했는가? |
| count_detected_object | 인식한 사물의 수 |
| is_detected_among_objects | 사물 중 □ 을(를) 인식했는가? |

### block_ai_utilize_pose_landmarker.js (50개, 그 중 고유 50개) 

| 블럭 ID | 블럭 이름 |
|---|---|
| audio_title | □ |
| check_microphone | 마이크가 연결되었는가? |
| get_microphone_volume | 마이크 소리 크기 |
| speech_to_text_title | □ |
| speech_to_text_convert | □ 음성 인식하기 □ |
| timed_speech_to_text_convert | □ 초 동안 □ 음성 인식하기 □ |
| set_visible_speech_to_text | 인식한 음성 □ □ |
| speech_to_text_get_value | 음성을 문자로 바꾼 값 |
| face_landmarker_title | □ |
| when_face_landmarker | □ 얼굴을 인식했을 때 |
| face_landmarker | 얼굴 인식 □ □ |
| draw_detected_face | 인식한 얼굴 □ □ |
| check_detected_face | 얼굴을 인식했는가? |
| count_detected_face | 인식한 얼굴의 수 |
| locate_to_face | □ 번째의 얼굴의 □ (으)로 이동하기 □ |
| locate_time_to_face | □ 초 동안 □ 번째의 얼굴의 □ (으)로 이동하기 □ |
| check_detected_gender | □ 번째 얼굴의 성별이 □ 인가? |
| check_compare_age | □ 번째 얼굴의 나이 □ □ 인가? |
| check_detected_emotion | □ 번째 얼굴의 감정이 □ 인가? |
| axis_detected_face | □ 번째 얼굴의 □ 의 □ 좌표 |
| get_detected_face_value | □ 번째 얼굴의 □ |
| hand_detection_title | □ |
| when_hand_detection | □ 손을 인식했을 때 |
| hand_detection | 손 인식 □ □ |
| draw_detected_hand | 인식한 손 □ □ |
| check_detected_hand | 손을 인식했는가? |
| count_detected_hand | 인식한 손의 수 |
| locate_to_hand | □ 번째 손의 □ □ (으)로 이동하기 □ |
| locate_time_to_hand | □ 초 동안 □ 번째 손의 □ □ (으)로 이동하기 □ |
| axis_detected_hand | □ 번째 손의 □ □ 의 □ 좌표 |
| is_which_hand | □ 번째 손이 □ 인가? |
| get_which_hand | □ 번째 손 |
| is_which_gesture | □ 번째 손의 모양이 □ 인가? |
| get_which_gesture | □ 번째 손의 모양 |
| object_detector_title | □ |
| when_object_detector | □ 사물을 인식했을 때 |
| object_detector | 사물 인식 □ □ |
| draw_detected_object | 인식한 사물 □ □ |
| check_detected_object | 사물을 인식했는가? |
| count_detected_object | 인식한 사물의 수 |
| is_detected_among_objects | 사물 중 □ 을(를) 인식했는가? |
| pose_landmarker_title | □ |
| when_pose_landmarker | □ 사람을 인식했을 때 |
| pose_landmarker | 사람 인식 □ □ |
| draw_detected_pose | 인식한 사람 □ □ |
| check_detected_pose | 사람을 인식했는가? |
| count_detected_pose | 인식한 사람의 수 |
| locate_to_pose | □ 번째의 사람의 □ (으)로 이동하기 □ |
| locate_time_to_pose | □ 초 동안 □ 번째의 사람의 □ (으)로 이동하기 □ |
| axis_detected_pose | □ 번째 사람의 □ 의 □ 좌표 |

### block_ai_utilize_translate.js (53개, 그 중 고유 53개) 

| 블럭 ID | 블럭 이름 |
|---|---|
| audio_title | □ |
| check_microphone | 마이크가 연결되었는가? |
| get_microphone_volume | 마이크 소리 크기 |
| speech_to_text_title | □ |
| speech_to_text_convert | □ 음성 인식하기 □ |
| timed_speech_to_text_convert | □ 초 동안 □ 음성 인식하기 □ |
| set_visible_speech_to_text | 인식한 음성 □ □ |
| speech_to_text_get_value | 음성을 문자로 바꾼 값 |
| face_landmarker_title | □ |
| when_face_landmarker | □ 얼굴을 인식했을 때 |
| face_landmarker | 얼굴 인식 □ □ |
| draw_detected_face | 인식한 얼굴 □ □ |
| check_detected_face | 얼굴을 인식했는가? |
| count_detected_face | 인식한 얼굴의 수 |
| locate_to_face | □ 번째의 얼굴의 □ (으)로 이동하기 □ |
| locate_time_to_face | □ 초 동안 □ 번째의 얼굴의 □ (으)로 이동하기 □ |
| check_detected_gender | □ 번째 얼굴의 성별이 □ 인가? |
| check_compare_age | □ 번째 얼굴의 나이 □ □ 인가? |
| check_detected_emotion | □ 번째 얼굴의 감정이 □ 인가? |
| axis_detected_face | □ 번째 얼굴의 □ 의 □ 좌표 |
| get_detected_face_value | □ 번째 얼굴의 □ |
| hand_detection_title | □ |
| when_hand_detection | □ 손을 인식했을 때 |
| hand_detection | 손 인식 □ □ |
| draw_detected_hand | 인식한 손 □ □ |
| check_detected_hand | 손을 인식했는가? |
| count_detected_hand | 인식한 손의 수 |
| locate_to_hand | □ 번째 손의 □ □ (으)로 이동하기 □ |
| locate_time_to_hand | □ 초 동안 □ 번째 손의 □ □ (으)로 이동하기 □ |
| axis_detected_hand | □ 번째 손의 □ □ 의 □ 좌표 |
| is_which_hand | □ 번째 손이 □ 인가? |
| get_which_hand | □ 번째 손 |
| is_which_gesture | □ 번째 손의 모양이 □ 인가? |
| get_which_gesture | □ 번째 손의 모양 |
| object_detector_title | □ |
| when_object_detector | □ 사물을 인식했을 때 |
| object_detector | 사물 인식 □ □ |
| draw_detected_object | 인식한 사물 □ □ |
| check_detected_object | 사물을 인식했는가? |
| count_detected_object | 인식한 사물의 수 |
| is_detected_among_objects | 사물 중 □ 을(를) 인식했는가? |
| pose_landmarker_title | □ |
| when_pose_landmarker | □ 사람을 인식했을 때 |
| pose_landmarker | 사람 인식 □ □ |
| draw_detected_pose | 인식한 사람 □ □ |
| check_detected_pose | 사람을 인식했는가? |
| count_detected_pose | 인식한 사람의 수 |
| locate_to_pose | □ 번째의 사람의 □ (으)로 이동하기 □ |
| locate_time_to_pose | □ 초 동안 □ 번째의 사람의 □ (으)로 이동하기 □ |
| axis_detected_pose | □ 번째 사람의 □ 의 □ 좌표 |
| translate_title | □ |
| get_translated_string | □ □ 을(를) □(으)로 번역한 값 |
| check_language | □의 언어 |

### block_ai_utilize_tts.js (57개, 그 중 고유 57개) 

| 블럭 ID | 블럭 이름 |
|---|---|
| audio_title | □ |
| check_microphone | 마이크가 연결되었는가? |
| get_microphone_volume | 마이크 소리 크기 |
| speech_to_text_title | □ |
| speech_to_text_convert | □ 음성 인식하기 □ |
| timed_speech_to_text_convert | □ 초 동안 □ 음성 인식하기 □ |
| set_visible_speech_to_text | 인식한 음성 □ □ |
| speech_to_text_get_value | 음성을 문자로 바꾼 값 |
| face_landmarker_title | □ |
| when_face_landmarker | □ 얼굴을 인식했을 때 |
| face_landmarker | 얼굴 인식 □ □ |
| draw_detected_face | 인식한 얼굴 □ □ |
| check_detected_face | 얼굴을 인식했는가? |
| count_detected_face | 인식한 얼굴의 수 |
| locate_to_face | □ 번째의 얼굴의 □ (으)로 이동하기 □ |
| locate_time_to_face | □ 초 동안 □ 번째의 얼굴의 □ (으)로 이동하기 □ |
| check_detected_gender | □ 번째 얼굴의 성별이 □ 인가? |
| check_compare_age | □ 번째 얼굴의 나이 □ □ 인가? |
| check_detected_emotion | □ 번째 얼굴의 감정이 □ 인가? |
| axis_detected_face | □ 번째 얼굴의 □ 의 □ 좌표 |
| get_detected_face_value | □ 번째 얼굴의 □ |
| hand_detection_title | □ |
| when_hand_detection | □ 손을 인식했을 때 |
| hand_detection | 손 인식 □ □ |
| draw_detected_hand | 인식한 손 □ □ |
| check_detected_hand | 손을 인식했는가? |
| count_detected_hand | 인식한 손의 수 |
| locate_to_hand | □ 번째 손의 □ □ (으)로 이동하기 □ |
| locate_time_to_hand | □ 초 동안 □ 번째 손의 □ □ (으)로 이동하기 □ |
| axis_detected_hand | □ 번째 손의 □ □ 의 □ 좌표 |
| is_which_hand | □ 번째 손이 □ 인가? |
| get_which_hand | □ 번째 손 |
| is_which_gesture | □ 번째 손의 모양이 □ 인가? |
| get_which_gesture | □ 번째 손의 모양 |
| object_detector_title | □ |
| when_object_detector | □ 사물을 인식했을 때 |
| object_detector | 사물 인식 □ □ |
| draw_detected_object | 인식한 사물 □ □ |
| check_detected_object | 사물을 인식했는가? |
| count_detected_object | 인식한 사물의 수 |
| is_detected_among_objects | 사물 중 □ 을(를) 인식했는가? |
| pose_landmarker_title | □ |
| when_pose_landmarker | □ 사람을 인식했을 때 |
| pose_landmarker | 사람 인식 □ □ |
| draw_detected_pose | 인식한 사람 □ □ |
| check_detected_pose | 사람을 인식했는가? |
| count_detected_pose | 인식한 사람의 수 |
| locate_to_pose | □ 번째의 사람의 □ (으)로 이동하기 □ |
| locate_time_to_pose | □ 초 동안 □ 번째의 사람의 □ (으)로 이동하기 □ |
| axis_detected_pose | □ 번째 사람의 □ 의 □ 좌표 |
| translate_title | □ |
| get_translated_string | □ □ 을(를) □(으)로 번역한 값 |
| check_language | □의 언어 |
| tts_title | □ |
| read_text | □ 읽어주기 □ |
| read_text_wait_with_block | □ 읽어주고 기다리기 □ |
| set_tts_property | □ 목소리를 □ 속도 □ 음높이로 설정하기 □ |

### block_ai_utilize_video.js (72개, 그 중 고유 72개) 

| 블럭 ID | 블럭 이름 |
|---|---|
| audio_title | □ |
| check_microphone | 마이크가 연결되었는가? |
| get_microphone_volume | 마이크 소리 크기 |
| speech_to_text_title | □ |
| speech_to_text_convert | □ 음성 인식하기 □ |
| timed_speech_to_text_convert | □ 초 동안 □ 음성 인식하기 □ |
| set_visible_speech_to_text | 인식한 음성 □ □ |
| speech_to_text_get_value | 음성을 문자로 바꾼 값 |
| face_landmarker_title | □ |
| when_face_landmarker | □ 얼굴을 인식했을 때 |
| face_landmarker | 얼굴 인식 □ □ |
| draw_detected_face | 인식한 얼굴 □ □ |
| check_detected_face | 얼굴을 인식했는가? |
| count_detected_face | 인식한 얼굴의 수 |
| locate_to_face | □ 번째의 얼굴의 □ (으)로 이동하기 □ |
| locate_time_to_face | □ 초 동안 □ 번째의 얼굴의 □ (으)로 이동하기 □ |
| check_detected_gender | □ 번째 얼굴의 성별이 □ 인가? |
| check_compare_age | □ 번째 얼굴의 나이 □ □ 인가? |
| check_detected_emotion | □ 번째 얼굴의 감정이 □ 인가? |
| axis_detected_face | □ 번째 얼굴의 □ 의 □ 좌표 |
| get_detected_face_value | □ 번째 얼굴의 □ |
| hand_detection_title | □ |
| when_hand_detection | □ 손을 인식했을 때 |
| hand_detection | 손 인식 □ □ |
| draw_detected_hand | 인식한 손 □ □ |
| check_detected_hand | 손을 인식했는가? |
| count_detected_hand | 인식한 손의 수 |
| locate_to_hand | □ 번째 손의 □ □ (으)로 이동하기 □ |
| locate_time_to_hand | □ 초 동안 □ 번째 손의 □ □ (으)로 이동하기 □ |
| axis_detected_hand | □ 번째 손의 □ □ 의 □ 좌표 |
| is_which_hand | □ 번째 손이 □ 인가? |
| get_which_hand | □ 번째 손 |
| is_which_gesture | □ 번째 손의 모양이 □ 인가? |
| get_which_gesture | □ 번째 손의 모양 |
| object_detector_title | □ |
| when_object_detector | □ 사물을 인식했을 때 |
| object_detector | 사물 인식 □ □ |
| draw_detected_object | 인식한 사물 □ □ |
| check_detected_object | 사물을 인식했는가? |
| count_detected_object | 인식한 사물의 수 |
| is_detected_among_objects | 사물 중 □ 을(를) 인식했는가? |
| pose_landmarker_title | □ |
| when_pose_landmarker | □ 사람을 인식했을 때 |
| pose_landmarker | 사람 인식 □ □ |
| draw_detected_pose | 인식한 사람 □ □ |
| check_detected_pose | 사람을 인식했는가? |
| count_detected_pose | 인식한 사람의 수 |
| locate_to_pose | □ 번째의 사람의 □ (으)로 이동하기 □ |
| locate_time_to_pose | □ 초 동안 □ 번째의 사람의 □ (으)로 이동하기 □ |
| axis_detected_pose | □ 번째 사람의 □ 의 □ 좌표 |
| translate_title | □ |
| get_translated_string | □ □ 을(를) □(으)로 번역한 값 |
| check_language | □의 언어 |
| tts_title | □ |
| read_text | □ 읽어주기 □ |
| read_text_wait_with_block | □ 읽어주고 기다리기 □ |
| set_tts_property | □ 목소리를 □ 속도 □ 음높이로 설정하기 □ |
| video_title | □ |
| video_change_cam | □ 카메라로 바꾸기 □ |
| video_check_webcam | 비디오가 연결되었는가? |
| video_draw_webcam | 비디오 화면 □ □ |
| video_set_camera_opacity_option | 비디오 투명도 효과를 □ 으로 정하기 □ |
| video_flip_camera | 비디오 화면 □ 뒤집기 □ |
| video_toggle_model | □ 인식 □ □ |
| video_toggle_ind | 인식된 □ □ □ |
| video_number_detect | 인식된 □ 의 수 |
| video_object_detected | 사물 중 □ (이)가 인식되었는가? |
| video_is_model_loaded | □ 인식이 되었는가? |
| video_detected_face_info | □ 번째 얼굴의 □ |
| video_motion_value | □ 에서 감지한 □ 값 |
| video_face_part_coord | □ 번째 얼굴의 □ 의 □ 좌표 |
| video_body_part_coord | □ 번째 사람의 □ 의 □ 좌표 |

## 5. 확장(교과) 블럭 (42개 고유)

### block_expansion_behaviorconduct_disaster.js (3개, 그 중 고유 3개)

| 블럭 ID | 블럭 이름 |
|---|---|
| behaviorConductDisaster_title | □ |
| count_disaster_behavior | □ □ 해야할 행동요령 수 |
| get_disaster_behavior | 자연재난□ □ 해야할 행동요령 □ 번째 항목 |

### block_expansion_behaviorconduct_lifesafety.js (6개, 그 중 고유 6개)

| 블럭 ID | 블럭 이름 |
|---|---|
| behaviorConductDisaster_title | □ |
| count_disaster_behavior | □ □ 해야할 행동요령 수 |
| get_disaster_behavior | 자연재난□ □ 해야할 행동요령 □ 번째 항목 |
| behaviorConductLifeSafety_title | □ |
| count_lifeSafety_behavior | □ 에서 □ 방법의 수 |
| get_lifeSafety_behavior | □ 에서 □ 방법 □ 번째 항목 |

### block_expansion_disasterAlert.js (10개, 그 중 고유 10개)

| 블럭 ID | 블럭 이름 |
|---|---|
| behaviorConductDisaster_title | □ |
| count_disaster_behavior | □ □ 해야할 행동요령 수 |
| get_disaster_behavior | 자연재난□ □ 해야할 행동요령 □ 번째 항목 |
| behaviorConductLifeSafety_title | □ |
| count_lifeSafety_behavior | □ 에서 □ 방법의 수 |
| get_lifeSafety_behavior | □ 에서 □ 방법 □ 번째 항목 |
| disaster_alert_title | (이름 없음) |
| count_disaster_alert | (이름 없음) |
| get_disaster_alert | (이름 없음) |
| check_disaster_alert | (이름 없음) |

### block_expansion_emergencyActionGuidelines.js (17개, 그 중 고유 17개)

| 블럭 ID | 블럭 이름 |
|---|---|
| behaviorConductDisaster_title | □ |
| count_disaster_behavior | □ □ 해야할 행동요령 수 |
| get_disaster_behavior | 자연재난□ □ 해야할 행동요령 □ 번째 항목 |
| behaviorConductLifeSafety_title | □ |
| count_lifeSafety_behavior | □ 에서 □ 방법의 수 |
| get_lifeSafety_behavior | □ 에서 □ 방법 □ 번째 항목 |
| disaster_alert_title | (이름 없음) |
| count_disaster_alert | (이름 없음) |
| get_disaster_alert | (이름 없음) |
| check_disaster_alert | (이름 없음) |
| emergencyActionGuidelines_title | (이름 없음) |
| count_disaster_guideline | 자연재난 □ □ 의 행동요령 수 |
| get_disaster_guideline | 자연재난 □ □ 의 행동요령 □ 번째 항목 |
| count_social_disaster_guideline | 사회재난 □ □ 의 행동요령 수 |
| get_social_disaster_guideline | 사회재난 □ □ 의 행동요령 □ 번째 항목 |
| count_safety_accident_guideline | 생활안전 □ □ 의 행동요령 수 |
| get_safety_accident_guideline | 생활안전 □ □ 의 행동요령 □ 번째 항목 |

### block_expansion_festival.js (20개, 그 중 고유 20개)

| 블럭 ID | 블럭 이름 |
|---|---|
| behaviorConductDisaster_title | □ |
| count_disaster_behavior | □ □ 해야할 행동요령 수 |
| get_disaster_behavior | 자연재난□ □ 해야할 행동요령 □ 번째 항목 |
| behaviorConductLifeSafety_title | □ |
| count_lifeSafety_behavior | □ 에서 □ 방법의 수 |
| get_lifeSafety_behavior | □ 에서 □ 방법 □ 번째 항목 |
| disaster_alert_title | (이름 없음) |
| count_disaster_alert | (이름 없음) |
| get_disaster_alert | (이름 없음) |
| check_disaster_alert | (이름 없음) |
| emergencyActionGuidelines_title | (이름 없음) |
| count_disaster_guideline | 자연재난 □ □ 의 행동요령 수 |
| get_disaster_guideline | 자연재난 □ □ 의 행동요령 □ 번째 항목 |
| count_social_disaster_guideline | 사회재난 □ □ 의 행동요령 수 |
| get_social_disaster_guideline | 사회재난 □ □ 의 행동요령 □ 번째 항목 |
| count_safety_accident_guideline | 생활안전 □ □ 의 행동요령 수 |
| get_safety_accident_guideline | 생활안전 □ □ 의 행동요령 □ 번째 항목 |
| festival_title | □ |
| count_festival | □ □ 행사의 수 |
| get_festival_info | □ □ 행사 □ 번째 항목의 □ |

### block_expansion_weather.js (42개, 그 중 고유 42개)

| 블럭 ID | 블럭 이름 |
|---|---|
| behaviorConductDisaster_title | □ |
| count_disaster_behavior | □ □ 해야할 행동요령 수 |
| get_disaster_behavior | 자연재난□ □ 해야할 행동요령 □ 번째 항목 |
| behaviorConductLifeSafety_title | □ |
| count_lifeSafety_behavior | □ 에서 □ 방법의 수 |
| get_lifeSafety_behavior | □ 에서 □ 방법 □ 번째 항목 |
| disaster_alert_title | (이름 없음) |
| count_disaster_alert | (이름 없음) |
| get_disaster_alert | (이름 없음) |
| check_disaster_alert | (이름 없음) |
| emergencyActionGuidelines_title | (이름 없음) |
| count_disaster_guideline | 자연재난 □ □ 의 행동요령 수 |
| get_disaster_guideline | 자연재난 □ □ 의 행동요령 □ 번째 항목 |
| count_social_disaster_guideline | 사회재난 □ □ 의 행동요령 수 |
| get_social_disaster_guideline | 사회재난 □ □ 의 행동요령 □ 번째 항목 |
| count_safety_accident_guideline | 생활안전 □ □ 의 행동요령 수 |
| get_safety_accident_guideline | 생활안전 □ □ 의 행동요령 □ 번째 항목 |
| festival_title | □ |
| count_festival | □ □ 행사의 수 |
| get_festival_info | □ □ 행사 □ 번째 항목의 □ |
| weather_title | □ |
| check_city_weather | □ □ □의 날씨가 □인가? |
| check_city_finedust | 현재 □ □ 의 미세먼지 등급이 □인가? |
| get_city_weather_data | □ □ □ 의 □ |
| get_current_city_weather_data | 현재 □ □ 의 □ |
| get_today_city_temperature | 오늘 □ □의 □시 기온 |
| check_weather | □ □ 의 날씨가 □인가? |
| check_finedust | 현재 □ 의 미세먼지 등급이 □인가? |
| get_weather_data | □ □ 의 □ |
| get_current_weather_data | 현재 □ 의 □ |
| get_today_temperature | 오늘 □의 □시 기온 |
| get_cur_weather | 현재 □의 날씨 |
| get_cur_wind | 현재 □의 풍향 |
| get_cur_weather_data | 현재 □의 □ |
| check_cur_weather | 현재 □의 날씨가 □인가? |
| check_cur_finddust | 현재 □의 미세먼지 등급이 □인가? |
| get_day_weather | □ □의 날씨 |
| get_day_weather_data | □ □의 □ |
| check_day_weather | □ □의 날씨가 □ 인가? |
| get_time_weather | □의 □시 날씨 |
| get_time_weather_data | □의 □시 □ |
| check_time_weather | □의 □시 날씨가 □ 인가? |

## 6. 범위 참고

- 기본/AI/확장 외에 하드웨어(202파일+lite 61파일) 블럭이 별도 존재(기기별 수천 개).
- AI 학습 하위 파일(클러스터/의사결정트리/KNN/회귀/SVM 등)은 공유 학습 블럭의 파라미터 구성이 다를 뿐 신규 블럭 ID가 없음.
