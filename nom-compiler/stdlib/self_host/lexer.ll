; ModuleID = 'nom_module'
source_filename = "nom_module"

%NomString = type { ptr, i64 }
%NomList = type { ptr, i64, i64 }
%Lexer = type { %NomString, i64, i64, i64 }
%Span = type { i64, i64, i64, i64 }
%Token = type { i8, [16 x i8] }
%SpannedToken = type { %Token, %Span }

@str = private unnamed_addr constant [1 x i8] zeroinitializer, align 1
@str.1 = private unnamed_addr constant [2 x i8] c"\0A\00", align 1
@str.2 = private unnamed_addr constant [2 x i8] c"\09\00", align 1
@str.3 = private unnamed_addr constant [2 x i8] c"\22\00", align 1
@str.4 = private unnamed_addr constant [2 x i8] c"\\\00", align 1
@str.5 = private unnamed_addr constant [7 x i8] c"system\00", align 1
@str.6 = private unnamed_addr constant [5 x i8] c"flow\00", align 1
@str.7 = private unnamed_addr constant [6 x i8] c"store\00", align 1
@str.8 = private unnamed_addr constant [6 x i8] c"graph\00", align 1
@str.9 = private unnamed_addr constant [6 x i8] c"agent\00", align 1
@str.10 = private unnamed_addr constant [5 x i8] c"test\00", align 1
@str.11 = private unnamed_addr constant [4 x i8] c"nom\00", align 1
@str.12 = private unnamed_addr constant [5 x i8] c"gate\00", align 1
@str.13 = private unnamed_addr constant [5 x i8] c"pool\00", align 1
@str.14 = private unnamed_addr constant [5 x i8] c"view\00", align 1
@str.15 = private unnamed_addr constant [5 x i8] c"need\00", align 1
@str.16 = private unnamed_addr constant [8 x i8] c"require\00", align 1
@str.17 = private unnamed_addr constant [8 x i8] c"effects\00", align 1
@str.18 = private unnamed_addr constant [6 x i8] c"where\00", align 1
@str.19 = private unnamed_addr constant [5 x i8] c"only\00", align 1
@str.20 = private unnamed_addr constant [9 x i8] c"describe\00", align 1
@str.21 = private unnamed_addr constant [7 x i8] c"branch\00", align 1
@str.22 = private unnamed_addr constant [7 x i8] c"iftrue\00", align 1
@str.23 = private unnamed_addr constant [8 x i8] c"iffalse\00", align 1
@str.24 = private unnamed_addr constant [6 x i8] c"given\00", align 1
@str.25 = private unnamed_addr constant [5 x i8] c"when\00", align 1
@str.26 = private unnamed_addr constant [5 x i8] c"then\00", align 1
@str.27 = private unnamed_addr constant [4 x i8] c"and\00", align 1
@str.28 = private unnamed_addr constant [3 x i8] c"or\00", align 1
@str.29 = private unnamed_addr constant [9 x i8] c"contract\00", align 1
@str.30 = private unnamed_addr constant [10 x i8] c"implement\00", align 1
@str.31 = private unnamed_addr constant [5 x i8] c"node\00", align 1
@str.32 = private unnamed_addr constant [5 x i8] c"edge\00", align 1
@str.33 = private unnamed_addr constant [6 x i8] c"query\00", align 1
@str.34 = private unnamed_addr constant [11 x i8] c"constraint\00", align 1
@str.35 = private unnamed_addr constant [11 x i8] c"capability\00", align 1
@str.36 = private unnamed_addr constant [10 x i8] c"supervise\00", align 1
@str.37 = private unnamed_addr constant [8 x i8] c"receive\00", align 1
@str.38 = private unnamed_addr constant [6 x i8] c"state\00", align 1
@str.39 = private unnamed_addr constant [9 x i8] c"schedule\00", align 1
@str.40 = private unnamed_addr constant [6 x i8] c"every\00", align 1
@str.41 = private unnamed_addr constant [4 x i8] c"let\00", align 1
@str.42 = private unnamed_addr constant [4 x i8] c"mut\00", align 1
@str.43 = private unnamed_addr constant [3 x i8] c"if\00", align 1
@str.44 = private unnamed_addr constant [5 x i8] c"else\00", align 1
@str.45 = private unnamed_addr constant [4 x i8] c"for\00", align 1
@str.46 = private unnamed_addr constant [6 x i8] c"while\00", align 1
@str.47 = private unnamed_addr constant [5 x i8] c"loop\00", align 1
@str.48 = private unnamed_addr constant [6 x i8] c"match\00", align 1
@str.49 = private unnamed_addr constant [7 x i8] c"return\00", align 1
@str.50 = private unnamed_addr constant [6 x i8] c"break\00", align 1
@str.51 = private unnamed_addr constant [9 x i8] c"continue\00", align 1
@str.52 = private unnamed_addr constant [3 x i8] c"fn\00", align 1
@str.53 = private unnamed_addr constant [5 x i8] c"type\00", align 1
@str.54 = private unnamed_addr constant [7 x i8] c"struct\00", align 1
@str.55 = private unnamed_addr constant [5 x i8] c"enum\00", align 1
@str.56 = private unnamed_addr constant [4 x i8] c"use\00", align 1
@str.57 = private unnamed_addr constant [4 x i8] c"pub\00", align 1
@str.58 = private unnamed_addr constant [3 x i8] c"in\00", align 1
@str.59 = private unnamed_addr constant [3 x i8] c"as\00", align 1
@str.60 = private unnamed_addr constant [4 x i8] c"mod\00", align 1
@str.61 = private unnamed_addr constant [6 x i8] c"trait\00", align 1
@str.62 = private unnamed_addr constant [5 x i8] c"impl\00", align 1
@str.63 = private unnamed_addr constant [5 x i8] c"self\00", align 1
@str.64 = private unnamed_addr constant [6 x i8] c"async\00", align 1
@str.65 = private unnamed_addr constant [6 x i8] c"await\00", align 1
@str.66 = private unnamed_addr constant [7 x i8] c"define\00", align 1
@str.67 = private unnamed_addr constant [5 x i8] c"kind\00", align 1
@str.68 = private unnamed_addr constant [7 x i8] c"choice\00", align 1
@str.69 = private unnamed_addr constant [5 x i8] c"take\00", align 1
@str.70 = private unnamed_addr constant [4 x i8] c"set\00", align 1
@str.71 = private unnamed_addr constant [9 x i8] c"changing\00", align 1
@str.72 = private unnamed_addr constant [5 x i8] c"give\00", align 1
@str.73 = private unnamed_addr constant [8 x i8] c"produce\00", align 1
@str.74 = private unnamed_addr constant [5 x i8] c"each\00", align 1
@str.75 = private unnamed_addr constant [7 x i8] c"repeat\00", align 1
@str.76 = private unnamed_addr constant [6 x i8] c"check\00", align 1
@str.77 = private unnamed_addr constant [6 x i8] c"share\00", align 1
@str.78 = private unnamed_addr constant [6 x i8] c"group\00", align 1
@str.79 = private unnamed_addr constant [9 x i8] c"behavior\00", align 1
@str.80 = private unnamed_addr constant [6 x i8] c"apply\00", align 1
@str.81 = private unnamed_addr constant [5 x i8] c"true\00", align 1
@str.82 = private unnamed_addr constant [6 x i8] c"false\00", align 1
@str.83 = private unnamed_addr constant [4 x i8] c"yes\00", align 1
@str.84 = private unnamed_addr constant [3 x i8] c"no\00", align 1
@str.85 = private unnamed_addr constant [5 x i8] c"none\00", align 1
@str.86 = private unnamed_addr constant [8 x i8] c"nothing\00", align 1
@str.87 = private unnamed_addr constant [8 x i8] c"unknown\00", align 1

declare void @nom_print(ptr, i64)

declare void @nom_println(ptr, i64)

declare void @nom_print_int(i64)

declare void @nom_print_float(double)

declare void @nom_print_bool(i8)

declare ptr @nom_alloc(i64)

declare void @nom_free(ptr, i64)

declare %NomString @nom_string_concat(ptr, ptr)

declare i32 @nom_string_eq(ptr, ptr)

declare %NomString @nom_string_slice(ptr, i64, i64)

declare ptr @nom_read_file(ptr, i64)

declare i32 @nom_write_file(ptr, i64, ptr, i64)

declare void @nom_panic(ptr, i64)

declare i64 @nom_parse_int(ptr)

declare double @nom_parse_float(ptr)

declare %NomString @nom_chr(i64)

declare %NomList @nom_list_new(i64)

declare %NomList @nom_list_with_capacity(i64, i64)

declare void @nom_list_push(ptr, ptr, i64)

declare ptr @nom_list_get(ptr, i64, i64)

declare i64 @nom_list_len(ptr)

declare void @nom_list_free_sized(ptr, i64)

define %Lexer @new_lexer(%NomString %source1) {
entry:
  %source = alloca %NomString, align 8
  store %NomString %source1, ptr %source, align 8
  %Lexer_init = alloca %Lexer, align 8
  store %Lexer zeroinitializer, ptr %Lexer_init, align 8
  %source2 = load %NomString, ptr %source, align 8
  %Lexer.source.init = getelementptr inbounds %Lexer, ptr %Lexer_init, i32 0, i32 0
  store %NomString %source2, ptr %Lexer.source.init, align 8
  %Lexer.pos.init = getelementptr inbounds %Lexer, ptr %Lexer_init, i32 0, i32 1
  store i64 0, ptr %Lexer.pos.init, align 4
  %Lexer.line.init = getelementptr inbounds %Lexer, ptr %Lexer_init, i32 0, i32 2
  store i64 1, ptr %Lexer.line.init, align 4
  %Lexer.col.init = getelementptr inbounds %Lexer, ptr %Lexer_init, i32 0, i32 3
  store i64 1, ptr %Lexer.col.init, align 4
  %Lexer_val = load %Lexer, ptr %Lexer_init, align 8
  ret %Lexer %Lexer_val
}

define i1 @is_at_end(%Lexer %lex1) {
entry:
  %lex = alloca %Lexer, align 8
  store %Lexer %lex1, ptr %lex, align 8
  %lex.pos.ptr = getelementptr inbounds %Lexer, ptr %lex, i32 0, i32 1
  %lex.pos = load i64, ptr %lex.pos.ptr, align 4
  %lex.source.ptr = getelementptr inbounds %Lexer, ptr %lex, i32 0, i32 0
  %lex.source = load %NomString, ptr %lex.source.ptr, align 8
  %str_data = extractvalue %NomString %lex.source, 0
  %str_len = extractvalue %NomString %lex.source, 1
  %icmp = icmp sge i64 %lex.pos, %str_len
  ret i1 %icmp
}

define i1 @is_alpha(i64 %ch1) {
entry:
  %ch = alloca i64, align 8
  store i64 %ch1, ptr %ch, align 4
  %ch2 = load i64, ptr %ch, align 4
  %icmp = icmp sge i64 %ch2, 65
  %ch3 = load i64, ptr %ch, align 4
  %icmp4 = icmp sle i64 %ch3, 90
  %and = and i1 %icmp, %icmp4
  %ch5 = load i64, ptr %ch, align 4
  %icmp6 = icmp sge i64 %ch5, 97
  %ch7 = load i64, ptr %ch, align 4
  %icmp8 = icmp sle i64 %ch7, 122
  %and9 = and i1 %icmp6, %icmp8
  %or = or i1 %and, %and9
  %ch10 = load i64, ptr %ch, align 4
  %icmp11 = icmp eq i64 %ch10, 95
  %or12 = or i1 %or, %icmp11
  ret i1 %or12
}

define i1 @is_digit(i64 %ch1) {
entry:
  %ch = alloca i64, align 8
  store i64 %ch1, ptr %ch, align 4
  %ch2 = load i64, ptr %ch, align 4
  %icmp = icmp sge i64 %ch2, 48
  %ch3 = load i64, ptr %ch, align 4
  %icmp4 = icmp sle i64 %ch3, 57
  %and = and i1 %icmp, %icmp4
  ret i1 %and
}

define i1 @is_alnum(i64 %ch1) {
entry:
  %ch = alloca i64, align 8
  store i64 %ch1, ptr %ch, align 4
  %ch2 = load i64, ptr %ch, align 4
  %call = call i1 @is_alpha(i64 %ch2)
  %ch3 = load i64, ptr %ch, align 4
  %call4 = call i1 @is_digit(i64 %ch3)
  %or = or i1 %call, %call4
  ret i1 %or
}

define i1 @is_whitespace(i64 %ch1) {
entry:
  %ch = alloca i64, align 8
  store i64 %ch1, ptr %ch, align 4
  %ch2 = load i64, ptr %ch, align 4
  %icmp = icmp eq i64 %ch2, 32
  %ch3 = load i64, ptr %ch, align 4
  %icmp4 = icmp eq i64 %ch3, 9
  %or = or i1 %icmp, %icmp4
  %ch5 = load i64, ptr %ch, align 4
  %icmp6 = icmp eq i64 %ch5, 13
  %or7 = or i1 %or, %icmp6
  ret i1 %or7
}

define i64 @current_char(%Lexer %lex1) {
entry:
  %lex = alloca %Lexer, align 8
  store %Lexer %lex1, ptr %lex, align 8
  %lex.source.ptr = getelementptr inbounds %Lexer, ptr %lex, i32 0, i32 0
  %lex.source = load %NomString, ptr %lex.source.ptr, align 8
  %lex.pos.ptr = getelementptr inbounds %Lexer, ptr %lex, i32 0, i32 1
  %lex.pos = load i64, ptr %lex.pos.ptr, align 4
  %str_data = extractvalue %NomString %lex.source, 0
  %str_len = extractvalue %NomString %lex.source, 1
  %str_idx_ptr = getelementptr inbounds i8, ptr %str_data, i64 %lex.pos
  %str_byte = load i8, ptr %str_idx_ptr, align 1
  %str_byte_ext = zext i8 %str_byte to i64
  ret i64 %str_byte_ext
}

define i64 @peek_next(%Lexer %lex1) {
entry:
  %lex = alloca %Lexer, align 8
  store %Lexer %lex1, ptr %lex, align 8
  %lex.pos.ptr = getelementptr inbounds %Lexer, ptr %lex, i32 0, i32 1
  %lex.pos = load i64, ptr %lex.pos.ptr, align 4
  %iadd = add i64 %lex.pos, 1
  %lex.source.ptr = getelementptr inbounds %Lexer, ptr %lex, i32 0, i32 0
  %lex.source = load %NomString, ptr %lex.source.ptr, align 8
  %str_data = extractvalue %NomString %lex.source, 0
  %str_len = extractvalue %NomString %lex.source, 1
  %icmp = icmp sge i64 %iadd, %str_len
  br i1 %icmp, label %then, label %else

then:                                             ; preds = %entry
  ret i64 0

else:                                             ; preds = %entry
  br label %ifcont

ifcont:                                           ; preds = %else
  %lex.source.ptr2 = getelementptr inbounds %Lexer, ptr %lex, i32 0, i32 0
  %lex.source3 = load %NomString, ptr %lex.source.ptr2, align 8
  %lex.pos.ptr4 = getelementptr inbounds %Lexer, ptr %lex, i32 0, i32 1
  %lex.pos5 = load i64, ptr %lex.pos.ptr4, align 4
  %iadd6 = add i64 %lex.pos5, 1
  %str_data7 = extractvalue %NomString %lex.source3, 0
  %str_len8 = extractvalue %NomString %lex.source3, 1
  %str_idx_ptr = getelementptr inbounds i8, ptr %str_data7, i64 %iadd6
  %str_byte = load i8, ptr %str_idx_ptr, align 1
  %str_byte_ext = zext i8 %str_byte to i64
  ret i64 %str_byte_ext
}

define %Lexer @advance(%Lexer %lex1) {
entry:
  %lex = alloca %Lexer, align 8
  store %Lexer %lex1, ptr %lex, align 8
  %lex2 = load %Lexer, ptr %lex, align 8
  %call = call i64 @current_char(%Lexer %lex2)
  %ch = alloca i64, align 8
  store i64 %call, ptr %ch, align 4
  %lex.line.ptr = getelementptr inbounds %Lexer, ptr %lex, i32 0, i32 2
  %lex.line = load i64, ptr %lex.line.ptr, align 4
  %new_line = alloca i64, align 8
  store i64 %lex.line, ptr %new_line, align 4
  %lex.col.ptr = getelementptr inbounds %Lexer, ptr %lex, i32 0, i32 3
  %lex.col = load i64, ptr %lex.col.ptr, align 4
  %iadd = add i64 %lex.col, 1
  %new_col = alloca i64, align 8
  store i64 %iadd, ptr %new_col, align 4
  %ch3 = load i64, ptr %ch, align 4
  %icmp = icmp eq i64 %ch3, 10
  br i1 %icmp, label %then, label %else

then:                                             ; preds = %entry
  %lex.line.ptr4 = getelementptr inbounds %Lexer, ptr %lex, i32 0, i32 2
  %lex.line5 = load i64, ptr %lex.line.ptr4, align 4
  %iadd6 = add i64 %lex.line5, 1
  store i64 %iadd6, ptr %new_line, align 4
  store i64 1, ptr %new_col, align 4
  br label %ifcont

else:                                             ; preds = %entry
  br label %ifcont

ifcont:                                           ; preds = %else, %then
  %Lexer_init = alloca %Lexer, align 8
  store %Lexer zeroinitializer, ptr %Lexer_init, align 8
  %lex.source.ptr = getelementptr inbounds %Lexer, ptr %lex, i32 0, i32 0
  %lex.source = load %NomString, ptr %lex.source.ptr, align 8
  %Lexer.source.init = getelementptr inbounds %Lexer, ptr %Lexer_init, i32 0, i32 0
  store %NomString %lex.source, ptr %Lexer.source.init, align 8
  %lex.pos.ptr = getelementptr inbounds %Lexer, ptr %lex, i32 0, i32 1
  %lex.pos = load i64, ptr %lex.pos.ptr, align 4
  %iadd7 = add i64 %lex.pos, 1
  %Lexer.pos.init = getelementptr inbounds %Lexer, ptr %Lexer_init, i32 0, i32 1
  store i64 %iadd7, ptr %Lexer.pos.init, align 4
  %new_line8 = load i64, ptr %new_line, align 4
  %Lexer.line.init = getelementptr inbounds %Lexer, ptr %Lexer_init, i32 0, i32 2
  store i64 %new_line8, ptr %Lexer.line.init, align 4
  %new_col9 = load i64, ptr %new_col, align 4
  %Lexer.col.init = getelementptr inbounds %Lexer, ptr %Lexer_init, i32 0, i32 3
  store i64 %new_col9, ptr %Lexer.col.init, align 4
  %Lexer_val = load %Lexer, ptr %Lexer_init, align 8
  ret %Lexer %Lexer_val
}

define %Span @make_span(i64 %start_pos1, i64 %start_line2, i64 %start_col3, %Lexer %lex4) {
entry:
  %start_pos = alloca i64, align 8
  store i64 %start_pos1, ptr %start_pos, align 4
  %start_line = alloca i64, align 8
  store i64 %start_line2, ptr %start_line, align 4
  %start_col = alloca i64, align 8
  store i64 %start_col3, ptr %start_col, align 4
  %lex = alloca %Lexer, align 8
  store %Lexer %lex4, ptr %lex, align 8
  %Span_init = alloca %Span, align 8
  store %Span zeroinitializer, ptr %Span_init, align 4
  %start_pos5 = load i64, ptr %start_pos, align 4
  %Span.start.init = getelementptr inbounds %Span, ptr %Span_init, i32 0, i32 0
  store i64 %start_pos5, ptr %Span.start.init, align 4
  %lex.pos.ptr = getelementptr inbounds %Lexer, ptr %lex, i32 0, i32 1
  %lex.pos = load i64, ptr %lex.pos.ptr, align 4
  %Span.end.init = getelementptr inbounds %Span, ptr %Span_init, i32 0, i32 1
  store i64 %lex.pos, ptr %Span.end.init, align 4
  %start_line6 = load i64, ptr %start_line, align 4
  %Span.line.init = getelementptr inbounds %Span, ptr %Span_init, i32 0, i32 2
  store i64 %start_line6, ptr %Span.line.init, align 4
  %start_col7 = load i64, ptr %start_col, align 4
  %Span.col.init = getelementptr inbounds %Span, ptr %Span_init, i32 0, i32 3
  store i64 %start_col7, ptr %Span.col.init, align 4
  %Span_val = load %Span, ptr %Span_init, align 4
  ret %Span %Span_val
}

define %Lexer @skip_horizontal_ws(%Lexer %lex1) {
entry:
  %lex = alloca %Lexer, align 8
  store %Lexer %lex1, ptr %lex, align 8
  %lex2 = load %Lexer, ptr %lex, align 8
  %l = alloca %Lexer, align 8
  store %Lexer %lex2, ptr %l, align 8
  br label %loop

loop:                                             ; preds = %ifcont, %entry
  %l3 = load %Lexer, ptr %l, align 8
  %call = call i1 @is_at_end(%Lexer %l3)
  %not = xor i1 %call, true
  br i1 %not, label %loopbody, label %loopend

loopbody:                                         ; preds = %loop
  %l4 = load %Lexer, ptr %l, align 8
  %call5 = call i64 @current_char(%Lexer %l4)
  %ch = alloca i64, align 8
  store i64 %call5, ptr %ch, align 4
  %ch6 = load i64, ptr %ch, align 4
  %call7 = call i1 @is_whitespace(i64 %ch6)
  br i1 %call7, label %then, label %else

loopend:                                          ; preds = %else, %loop
  %l10 = load %Lexer, ptr %l, align 8
  ret %Lexer %l10

then:                                             ; preds = %loopbody
  %l8 = load %Lexer, ptr %l, align 8
  %call9 = call %Lexer @advance(%Lexer %l8)
  store %Lexer %call9, ptr %l, align 8
  br label %ifcont

else:                                             ; preds = %loopbody
  br label %loopend

ifcont:                                           ; preds = %then
  br label %loop
}

define { %NomString, %Lexer } @read_comment(%Lexer %lex1) {
entry:
  %lex = alloca %Lexer, align 8
  store %Lexer %lex1, ptr %lex, align 8
  %lex2 = load %Lexer, ptr %lex, align 8
  %l = alloca %Lexer, align 8
  store %Lexer %lex2, ptr %l, align 8
  %l.pos.ptr = getelementptr inbounds %Lexer, ptr %l, i32 0, i32 1
  %l.pos = load i64, ptr %l.pos.ptr, align 4
  %start = alloca i64, align 8
  store i64 %l.pos, ptr %start, align 4
  br label %loop

loop:                                             ; preds = %loopbody, %entry
  %l3 = load %Lexer, ptr %l, align 8
  %call = call i1 @is_at_end(%Lexer %l3)
  %not = xor i1 %call, true
  %l4 = load %Lexer, ptr %l, align 8
  %call5 = call i64 @current_char(%Lexer %l4)
  %icmp = icmp ne i64 %call5, 10
  %and = and i1 %not, %icmp
  br i1 %and, label %loopbody, label %loopend

loopbody:                                         ; preds = %loop
  %l6 = load %Lexer, ptr %l, align 8
  %call7 = call %Lexer @advance(%Lexer %l6)
  store %Lexer %call7, ptr %l, align 8
  br label %loop

loopend:                                          ; preds = %loop
  %l.source.ptr = getelementptr inbounds %Lexer, ptr %l, i32 0, i32 0
  %l.source = load %NomString, ptr %l.source.ptr, align 8
  %start8 = load i64, ptr %start, align 4
  %l.pos.ptr9 = getelementptr inbounds %Lexer, ptr %l, i32 0, i32 1
  %l.pos10 = load i64, ptr %l.pos.ptr9, align 4
  %str_slot = alloca %NomString, align 8
  store %NomString %l.source, ptr %str_slot, align 8
  %str_slice = call %NomString @nom_string_slice(ptr %str_slot, i64 %start8, i64 %l.pos10)
  %content = alloca %NomString, align 8
  store %NomString %str_slice, ptr %content, align 8
  %content11 = load %NomString, ptr %content, align 8
  %l12 = load %Lexer, ptr %l, align 8
  %tup0 = insertvalue { %NomString, %Lexer } undef, %NomString %content11, 0
  %tup1 = insertvalue { %NomString, %Lexer } %tup0, %Lexer %l12, 1
  ret { %NomString, %Lexer } %tup1
}

define { %NomString, %Lexer } @read_word(%Lexer %lex1) {
entry:
  %lex = alloca %Lexer, align 8
  store %Lexer %lex1, ptr %lex, align 8
  %lex.pos.ptr = getelementptr inbounds %Lexer, ptr %lex, i32 0, i32 1
  %lex.pos = load i64, ptr %lex.pos.ptr, align 4
  %start = alloca i64, align 8
  store i64 %lex.pos, ptr %start, align 4
  %lex2 = load %Lexer, ptr %lex, align 8
  %l = alloca %Lexer, align 8
  store %Lexer %lex2, ptr %l, align 8
  br label %loop

loop:                                             ; preds = %loopbody, %entry
  %l3 = load %Lexer, ptr %l, align 8
  %call = call i1 @is_at_end(%Lexer %l3)
  %not = xor i1 %call, true
  %l4 = load %Lexer, ptr %l, align 8
  %call5 = call i64 @current_char(%Lexer %l4)
  %call6 = call i1 @is_alnum(i64 %call5)
  %and = and i1 %not, %call6
  br i1 %and, label %loopbody, label %loopend

loopbody:                                         ; preds = %loop
  %l7 = load %Lexer, ptr %l, align 8
  %call8 = call %Lexer @advance(%Lexer %l7)
  store %Lexer %call8, ptr %l, align 8
  br label %loop

loopend:                                          ; preds = %loop
  %l.source.ptr = getelementptr inbounds %Lexer, ptr %l, i32 0, i32 0
  %l.source = load %NomString, ptr %l.source.ptr, align 8
  %start9 = load i64, ptr %start, align 4
  %l.pos.ptr = getelementptr inbounds %Lexer, ptr %l, i32 0, i32 1
  %l.pos = load i64, ptr %l.pos.ptr, align 4
  %str_slot = alloca %NomString, align 8
  store %NomString %l.source, ptr %str_slot, align 8
  %str_slice = call %NomString @nom_string_slice(ptr %str_slot, i64 %start9, i64 %l.pos)
  %word = alloca %NomString, align 8
  store %NomString %str_slice, ptr %word, align 8
  %word10 = load %NomString, ptr %word, align 8
  %l11 = load %Lexer, ptr %l, align 8
  %tup0 = insertvalue { %NomString, %Lexer } undef, %NomString %word10, 0
  %tup1 = insertvalue { %NomString, %Lexer } %tup0, %Lexer %l11, 1
  ret { %NomString, %Lexer } %tup1
}

define { %Token, %Lexer } @read_number(%Lexer %lex1) {
entry:
  %lex = alloca %Lexer, align 8
  store %Lexer %lex1, ptr %lex, align 8
  %lex.pos.ptr = getelementptr inbounds %Lexer, ptr %lex, i32 0, i32 1
  %lex.pos = load i64, ptr %lex.pos.ptr, align 4
  %start = alloca i64, align 8
  store i64 %lex.pos, ptr %start, align 4
  %lex2 = load %Lexer, ptr %lex, align 8
  %l = alloca %Lexer, align 8
  store %Lexer %lex2, ptr %l, align 8
  %is_float = alloca i1, align 1
  store i1 false, ptr %is_float, align 1
  br label %loop

loop:                                             ; preds = %loopbody, %entry
  %l3 = load %Lexer, ptr %l, align 8
  %call = call i1 @is_at_end(%Lexer %l3)
  %not = xor i1 %call, true
  %l4 = load %Lexer, ptr %l, align 8
  %call5 = call i64 @current_char(%Lexer %l4)
  %call6 = call i1 @is_digit(i64 %call5)
  %and = and i1 %not, %call6
  br i1 %and, label %loopbody, label %loopend

loopbody:                                         ; preds = %loop
  %l7 = load %Lexer, ptr %l, align 8
  %call8 = call %Lexer @advance(%Lexer %l7)
  store %Lexer %call8, ptr %l, align 8
  br label %loop

loopend:                                          ; preds = %loop
  %l9 = load %Lexer, ptr %l, align 8
  %call10 = call i1 @is_at_end(%Lexer %l9)
  %not11 = xor i1 %call10, true
  %l12 = load %Lexer, ptr %l, align 8
  %call13 = call i64 @current_char(%Lexer %l12)
  %icmp = icmp eq i64 %call13, 46
  %and14 = and i1 %not11, %icmp
  br i1 %and14, label %then, label %else

then:                                             ; preds = %loopend
  %l15 = load %Lexer, ptr %l, align 8
  %call16 = call i64 @peek_next(%Lexer %l15)
  %after_dot = alloca i64, align 8
  store i64 %call16, ptr %after_dot, align 4
  %after_dot17 = load i64, ptr %after_dot, align 4
  %call18 = call i1 @is_digit(i64 %after_dot17)
  br i1 %call18, label %then19, label %else20

else:                                             ; preds = %loopend
  br label %ifcont

ifcont:                                           ; preds = %else, %ifcont21
  %l.source.ptr = getelementptr inbounds %Lexer, ptr %l, i32 0, i32 0
  %l.source = load %NomString, ptr %l.source.ptr, align 8
  %start36 = load i64, ptr %start, align 4
  %l.pos.ptr = getelementptr inbounds %Lexer, ptr %l, i32 0, i32 1
  %l.pos = load i64, ptr %l.pos.ptr, align 4
  %str_slot = alloca %NomString, align 8
  store %NomString %l.source, ptr %str_slot, align 8
  %str_slice = call %NomString @nom_string_slice(ptr %str_slot, i64 %start36, i64 %l.pos)
  %num_str = alloca %NomString, align 8
  store %NomString %str_slice, ptr %num_str, align 8
  %is_float37 = load i1, ptr %is_float, align 1
  br i1 %is_float37, label %then38, label %else39

then19:                                           ; preds = %then
  store i1 true, ptr %is_float, align 1
  %l22 = load %Lexer, ptr %l, align 8
  %call23 = call %Lexer @advance(%Lexer %l22)
  store %Lexer %call23, ptr %l, align 8
  br label %loop24

else20:                                           ; preds = %then
  br label %ifcont21

ifcont21:                                         ; preds = %else20, %loopend26
  br label %ifcont

loop24:                                           ; preds = %loopbody25, %then19
  %l27 = load %Lexer, ptr %l, align 8
  %call28 = call i1 @is_at_end(%Lexer %l27)
  %not29 = xor i1 %call28, true
  %l30 = load %Lexer, ptr %l, align 8
  %call31 = call i64 @current_char(%Lexer %l30)
  %call32 = call i1 @is_digit(i64 %call31)
  %and33 = and i1 %not29, %call32
  br i1 %and33, label %loopbody25, label %loopend26

loopbody25:                                       ; preds = %loop24
  %l34 = load %Lexer, ptr %l, align 8
  %call35 = call %Lexer @advance(%Lexer %l34)
  store %Lexer %call35, ptr %l, align 8
  br label %loop24

loopend26:                                        ; preds = %loop24
  br label %ifcont21

then38:                                           ; preds = %ifcont
  %num_str41 = load %NomString, ptr %num_str, align 8
  %str_slot42 = alloca %NomString, align 8
  store %NomString %num_str41, ptr %str_slot42, align 8
  %parse_float_call = call double @nom_parse_float(ptr %str_slot42)
  %Token_ctor = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor, align 1
  %tag_ptr = getelementptr inbounds %Token, ptr %Token_ctor, i32 0, i32 0
  store i8 91, ptr %tag_ptr, align 1
  %payload_ptr = getelementptr inbounds %Token, ptr %Token_ctor, i32 0, i32 1
  store double %parse_float_call, ptr %payload_ptr, align 8
  %enum_val = load %Token, ptr %Token_ctor, align 1
  %l43 = load %Lexer, ptr %l, align 8
  %tup0 = insertvalue { %Token, %Lexer } undef, %Token %enum_val, 0
  %tup1 = insertvalue { %Token, %Lexer } %tup0, %Lexer %l43, 1
  ret { %Token, %Lexer } %tup1

else39:                                           ; preds = %ifcont
  %num_str44 = load %NomString, ptr %num_str, align 8
  %str_slot45 = alloca %NomString, align 8
  store %NomString %num_str44, ptr %str_slot45, align 8
  %parse_int_call = call i64 @nom_parse_int(ptr %str_slot45)
  %Token_ctor46 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor46, align 1
  %tag_ptr47 = getelementptr inbounds %Token, ptr %Token_ctor46, i32 0, i32 0
  store i8 90, ptr %tag_ptr47, align 1
  %payload_ptr48 = getelementptr inbounds %Token, ptr %Token_ctor46, i32 0, i32 1
  store i64 %parse_int_call, ptr %payload_ptr48, align 4
  %enum_val49 = load %Token, ptr %Token_ctor46, align 1
  %l50 = load %Lexer, ptr %l, align 8
  %tup051 = insertvalue { %Token, %Lexer } undef, %Token %enum_val49, 0
  %tup152 = insertvalue { %Token, %Lexer } %tup051, %Lexer %l50, 1
  ret { %Token, %Lexer } %tup152

ifcont40:                                         ; No predecessors!
  unreachable
}

define { %NomString, %Lexer } @read_string(%Lexer %lex1) {
entry:
  %lex = alloca %Lexer, align 8
  store %Lexer %lex1, ptr %lex, align 8
  %lex2 = load %Lexer, ptr %lex, align 8
  %l = alloca %Lexer, align 8
  store %Lexer %lex2, ptr %l, align 8
  %buf = alloca %NomString, align 8
  store %NomString { ptr @str, i64 0 }, ptr %buf, align 8
  br label %loop

loop:                                             ; preds = %ifcont, %entry
  %l3 = load %Lexer, ptr %l, align 8
  %call = call i1 @is_at_end(%Lexer %l3)
  %not = xor i1 %call, true
  %l4 = load %Lexer, ptr %l, align 8
  %call5 = call i64 @current_char(%Lexer %l4)
  %icmp = icmp ne i64 %call5, 34
  %and = and i1 %not, %icmp
  br i1 %and, label %loopbody, label %loopend

loopbody:                                         ; preds = %loop
  %l6 = load %Lexer, ptr %l, align 8
  %call7 = call i64 @current_char(%Lexer %l6)
  %ch = alloca i64, align 8
  store i64 %call7, ptr %ch, align 4
  %ch8 = load i64, ptr %ch, align 4
  %icmp9 = icmp eq i64 %ch8, 92
  br i1 %icmp9, label %then, label %else

loopend:                                          ; preds = %loop
  %l65 = load %Lexer, ptr %l, align 8
  %call66 = call i1 @is_at_end(%Lexer %l65)
  %not67 = xor i1 %call66, true
  br i1 %not67, label %then68, label %else69

then:                                             ; preds = %loopbody
  %l10 = load %Lexer, ptr %l, align 8
  %call11 = call %Lexer @advance(%Lexer %l10)
  store %Lexer %call11, ptr %l, align 8
  %l12 = load %Lexer, ptr %l, align 8
  %call13 = call i1 @is_at_end(%Lexer %l12)
  %not14 = xor i1 %call13, true
  br i1 %not14, label %then15, label %else16

else:                                             ; preds = %loopbody
  %buf56 = load %NomString, ptr %buf, align 8
  %ch57 = load i64, ptr %ch, align 4
  %chr_call58 = call %NomString @nom_chr(i64 %ch57)
  %str_slot59 = alloca %NomString, align 8
  store %NomString %buf56, ptr %str_slot59, align 8
  %str_slot60 = alloca %NomString, align 8
  store %NomString %chr_call58, ptr %str_slot60, align 8
  %str_concat61 = call %NomString @nom_string_concat(ptr %str_slot59, ptr %str_slot60)
  store %NomString %str_concat61, ptr %buf, align 8
  %l62 = load %Lexer, ptr %l, align 8
  %call63 = call %Lexer @advance(%Lexer %l62)
  store %Lexer %call63, ptr %l, align 8
  br label %ifcont

ifcont:                                           ; preds = %else, %ifcont17
  %iftmp64 = phi i8 [ 0, %ifcont17 ], [ 0, %else ]
  br label %loop

then15:                                           ; preds = %then
  %l18 = load %Lexer, ptr %l, align 8
  %call19 = call i64 @current_char(%Lexer %l18)
  %esc = alloca i64, align 8
  store i64 %call19, ptr %esc, align 4
  %esc20 = load i64, ptr %esc, align 4
  %icmp21 = icmp eq i64 %esc20, 110
  br i1 %icmp21, label %then22, label %else23

else16:                                           ; preds = %then
  br label %ifcont17

ifcont17:                                         ; preds = %else16, %ifcont24
  br label %ifcont

then22:                                           ; preds = %then15
  %buf25 = load %NomString, ptr %buf, align 8
  %str_slot = alloca %NomString, align 8
  store %NomString %buf25, ptr %str_slot, align 8
  %str_slot26 = alloca %NomString, align 8
  store %NomString { ptr @str.1, i64 1 }, ptr %str_slot26, align 8
  %str_concat = call %NomString @nom_string_concat(ptr %str_slot, ptr %str_slot26)
  store %NomString %str_concat, ptr %buf, align 8
  br label %ifcont24

else23:                                           ; preds = %then15
  %esc27 = load i64, ptr %esc, align 4
  %icmp28 = icmp eq i64 %esc27, 116
  br i1 %icmp28, label %elifthen, label %elifelse

ifcont24:                                         ; preds = %elifelse44, %elifthen43, %elifthen35, %elifthen, %then22
  %iftmp = phi i8 [ 0, %then22 ], [ 0, %elifthen ], [ 0, %elifthen35 ], [ 0, %elifthen43 ], [ 0, %elifelse44 ]
  %l54 = load %Lexer, ptr %l, align 8
  %call55 = call %Lexer @advance(%Lexer %l54)
  store %Lexer %call55, ptr %l, align 8
  br label %ifcont17

elifthen:                                         ; preds = %else23
  %buf29 = load %NomString, ptr %buf, align 8
  %str_slot30 = alloca %NomString, align 8
  store %NomString %buf29, ptr %str_slot30, align 8
  %str_slot31 = alloca %NomString, align 8
  store %NomString { ptr @str.2, i64 1 }, ptr %str_slot31, align 8
  %str_concat32 = call %NomString @nom_string_concat(ptr %str_slot30, ptr %str_slot31)
  store %NomString %str_concat32, ptr %buf, align 8
  br label %ifcont24

elifelse:                                         ; preds = %else23
  %esc33 = load i64, ptr %esc, align 4
  %icmp34 = icmp eq i64 %esc33, 34
  br i1 %icmp34, label %elifthen35, label %elifelse36

elifthen35:                                       ; preds = %elifelse
  %buf37 = load %NomString, ptr %buf, align 8
  %str_slot38 = alloca %NomString, align 8
  store %NomString %buf37, ptr %str_slot38, align 8
  %str_slot39 = alloca %NomString, align 8
  store %NomString { ptr @str.3, i64 1 }, ptr %str_slot39, align 8
  %str_concat40 = call %NomString @nom_string_concat(ptr %str_slot38, ptr %str_slot39)
  store %NomString %str_concat40, ptr %buf, align 8
  br label %ifcont24

elifelse36:                                       ; preds = %elifelse
  %esc41 = load i64, ptr %esc, align 4
  %icmp42 = icmp eq i64 %esc41, 92
  br i1 %icmp42, label %elifthen43, label %elifelse44

elifthen43:                                       ; preds = %elifelse36
  %buf45 = load %NomString, ptr %buf, align 8
  %str_slot46 = alloca %NomString, align 8
  store %NomString %buf45, ptr %str_slot46, align 8
  %str_slot47 = alloca %NomString, align 8
  store %NomString { ptr @str.4, i64 1 }, ptr %str_slot47, align 8
  %str_concat48 = call %NomString @nom_string_concat(ptr %str_slot46, ptr %str_slot47)
  store %NomString %str_concat48, ptr %buf, align 8
  br label %ifcont24

elifelse44:                                       ; preds = %elifelse36
  %buf49 = load %NomString, ptr %buf, align 8
  %esc50 = load i64, ptr %esc, align 4
  %chr_call = call %NomString @nom_chr(i64 %esc50)
  %str_slot51 = alloca %NomString, align 8
  store %NomString %buf49, ptr %str_slot51, align 8
  %str_slot52 = alloca %NomString, align 8
  store %NomString %chr_call, ptr %str_slot52, align 8
  %str_concat53 = call %NomString @nom_string_concat(ptr %str_slot51, ptr %str_slot52)
  store %NomString %str_concat53, ptr %buf, align 8
  br label %ifcont24

then68:                                           ; preds = %loopend
  %l71 = load %Lexer, ptr %l, align 8
  %call72 = call %Lexer @advance(%Lexer %l71)
  store %Lexer %call72, ptr %l, align 8
  br label %ifcont70

else69:                                           ; preds = %loopend
  br label %ifcont70

ifcont70:                                         ; preds = %else69, %then68
  %buf73 = load %NomString, ptr %buf, align 8
  %l74 = load %Lexer, ptr %l, align 8
  %tup0 = insertvalue { %NomString, %Lexer } undef, %NomString %buf73, 0
  %tup1 = insertvalue { %NomString, %Lexer } %tup0, %Lexer %l74, 1
  ret { %NomString, %Lexer } %tup1
}

define %Token @classify_word(%NomString %word1) {
entry:
  %word = alloca %NomString, align 8
  store %NomString %word1, ptr %word, align 8
  %word2 = load %NomString, ptr %word, align 8
  br label %match_test_0

match_end:                                        ; No predecessors!
  ret %Token zeroinitializer

match_test_0:                                     ; preds = %entry
  %str_slot = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot, align 8
  %str_slot3 = alloca %NomString, align 8
  store %NomString { ptr @str.5, i64 6 }, ptr %str_slot3, align 8
  %match_str_eq = call i32 @nom_string_eq(ptr %str_slot, ptr %str_slot3)
  %match_str_bool = icmp ne i32 %match_str_eq, 0
  br i1 %match_str_bool, label %match_arm_0, label %match_test_1

match_arm_0:                                      ; preds = %match_test_0
  %Token_ctor = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor, align 1
  %tag_ptr = getelementptr inbounds %Token, ptr %Token_ctor, i32 0, i32 0
  store i8 0, ptr %tag_ptr, align 1
  %enum_val = load %Token, ptr %Token_ctor, align 1
  ret %Token %enum_val

match_test_1:                                     ; preds = %match_test_0
  %str_slot4 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot4, align 8
  %str_slot5 = alloca %NomString, align 8
  store %NomString { ptr @str.6, i64 4 }, ptr %str_slot5, align 8
  %match_str_eq6 = call i32 @nom_string_eq(ptr %str_slot4, ptr %str_slot5)
  %match_str_bool7 = icmp ne i32 %match_str_eq6, 0
  br i1 %match_str_bool7, label %match_arm_1, label %match_test_2

match_arm_1:                                      ; preds = %match_test_1
  %Token_ctor8 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor8, align 1
  %tag_ptr9 = getelementptr inbounds %Token, ptr %Token_ctor8, i32 0, i32 0
  store i8 1, ptr %tag_ptr9, align 1
  %enum_val10 = load %Token, ptr %Token_ctor8, align 1
  ret %Token %enum_val10

match_test_2:                                     ; preds = %match_test_1
  %str_slot11 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot11, align 8
  %str_slot12 = alloca %NomString, align 8
  store %NomString { ptr @str.7, i64 5 }, ptr %str_slot12, align 8
  %match_str_eq13 = call i32 @nom_string_eq(ptr %str_slot11, ptr %str_slot12)
  %match_str_bool14 = icmp ne i32 %match_str_eq13, 0
  br i1 %match_str_bool14, label %match_arm_2, label %match_test_3

match_arm_2:                                      ; preds = %match_test_2
  %Token_ctor15 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor15, align 1
  %tag_ptr16 = getelementptr inbounds %Token, ptr %Token_ctor15, i32 0, i32 0
  store i8 2, ptr %tag_ptr16, align 1
  %enum_val17 = load %Token, ptr %Token_ctor15, align 1
  ret %Token %enum_val17

match_test_3:                                     ; preds = %match_test_2
  %str_slot18 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot18, align 8
  %str_slot19 = alloca %NomString, align 8
  store %NomString { ptr @str.8, i64 5 }, ptr %str_slot19, align 8
  %match_str_eq20 = call i32 @nom_string_eq(ptr %str_slot18, ptr %str_slot19)
  %match_str_bool21 = icmp ne i32 %match_str_eq20, 0
  br i1 %match_str_bool21, label %match_arm_3, label %match_test_4

match_arm_3:                                      ; preds = %match_test_3
  %Token_ctor22 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor22, align 1
  %tag_ptr23 = getelementptr inbounds %Token, ptr %Token_ctor22, i32 0, i32 0
  store i8 3, ptr %tag_ptr23, align 1
  %enum_val24 = load %Token, ptr %Token_ctor22, align 1
  ret %Token %enum_val24

match_test_4:                                     ; preds = %match_test_3
  %str_slot25 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot25, align 8
  %str_slot26 = alloca %NomString, align 8
  store %NomString { ptr @str.9, i64 5 }, ptr %str_slot26, align 8
  %match_str_eq27 = call i32 @nom_string_eq(ptr %str_slot25, ptr %str_slot26)
  %match_str_bool28 = icmp ne i32 %match_str_eq27, 0
  br i1 %match_str_bool28, label %match_arm_4, label %match_test_5

match_arm_4:                                      ; preds = %match_test_4
  %Token_ctor29 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor29, align 1
  %tag_ptr30 = getelementptr inbounds %Token, ptr %Token_ctor29, i32 0, i32 0
  store i8 4, ptr %tag_ptr30, align 1
  %enum_val31 = load %Token, ptr %Token_ctor29, align 1
  ret %Token %enum_val31

match_test_5:                                     ; preds = %match_test_4
  %str_slot32 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot32, align 8
  %str_slot33 = alloca %NomString, align 8
  store %NomString { ptr @str.10, i64 4 }, ptr %str_slot33, align 8
  %match_str_eq34 = call i32 @nom_string_eq(ptr %str_slot32, ptr %str_slot33)
  %match_str_bool35 = icmp ne i32 %match_str_eq34, 0
  br i1 %match_str_bool35, label %match_arm_5, label %match_test_6

match_arm_5:                                      ; preds = %match_test_5
  %Token_ctor36 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor36, align 1
  %tag_ptr37 = getelementptr inbounds %Token, ptr %Token_ctor36, i32 0, i32 0
  store i8 5, ptr %tag_ptr37, align 1
  %enum_val38 = load %Token, ptr %Token_ctor36, align 1
  ret %Token %enum_val38

match_test_6:                                     ; preds = %match_test_5
  %str_slot39 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot39, align 8
  %str_slot40 = alloca %NomString, align 8
  store %NomString { ptr @str.11, i64 3 }, ptr %str_slot40, align 8
  %match_str_eq41 = call i32 @nom_string_eq(ptr %str_slot39, ptr %str_slot40)
  %match_str_bool42 = icmp ne i32 %match_str_eq41, 0
  br i1 %match_str_bool42, label %match_arm_6, label %match_test_7

match_arm_6:                                      ; preds = %match_test_6
  %Token_ctor43 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor43, align 1
  %tag_ptr44 = getelementptr inbounds %Token, ptr %Token_ctor43, i32 0, i32 0
  store i8 6, ptr %tag_ptr44, align 1
  %enum_val45 = load %Token, ptr %Token_ctor43, align 1
  ret %Token %enum_val45

match_test_7:                                     ; preds = %match_test_6
  %str_slot46 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot46, align 8
  %str_slot47 = alloca %NomString, align 8
  store %NomString { ptr @str.12, i64 4 }, ptr %str_slot47, align 8
  %match_str_eq48 = call i32 @nom_string_eq(ptr %str_slot46, ptr %str_slot47)
  %match_str_bool49 = icmp ne i32 %match_str_eq48, 0
  br i1 %match_str_bool49, label %match_arm_7, label %match_test_8

match_arm_7:                                      ; preds = %match_test_7
  %Token_ctor50 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor50, align 1
  %tag_ptr51 = getelementptr inbounds %Token, ptr %Token_ctor50, i32 0, i32 0
  store i8 7, ptr %tag_ptr51, align 1
  %enum_val52 = load %Token, ptr %Token_ctor50, align 1
  ret %Token %enum_val52

match_test_8:                                     ; preds = %match_test_7
  %str_slot53 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot53, align 8
  %str_slot54 = alloca %NomString, align 8
  store %NomString { ptr @str.13, i64 4 }, ptr %str_slot54, align 8
  %match_str_eq55 = call i32 @nom_string_eq(ptr %str_slot53, ptr %str_slot54)
  %match_str_bool56 = icmp ne i32 %match_str_eq55, 0
  br i1 %match_str_bool56, label %match_arm_8, label %match_test_9

match_arm_8:                                      ; preds = %match_test_8
  %Token_ctor57 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor57, align 1
  %tag_ptr58 = getelementptr inbounds %Token, ptr %Token_ctor57, i32 0, i32 0
  store i8 8, ptr %tag_ptr58, align 1
  %enum_val59 = load %Token, ptr %Token_ctor57, align 1
  ret %Token %enum_val59

match_test_9:                                     ; preds = %match_test_8
  %str_slot60 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot60, align 8
  %str_slot61 = alloca %NomString, align 8
  store %NomString { ptr @str.14, i64 4 }, ptr %str_slot61, align 8
  %match_str_eq62 = call i32 @nom_string_eq(ptr %str_slot60, ptr %str_slot61)
  %match_str_bool63 = icmp ne i32 %match_str_eq62, 0
  br i1 %match_str_bool63, label %match_arm_9, label %match_test_10

match_arm_9:                                      ; preds = %match_test_9
  %Token_ctor64 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor64, align 1
  %tag_ptr65 = getelementptr inbounds %Token, ptr %Token_ctor64, i32 0, i32 0
  store i8 9, ptr %tag_ptr65, align 1
  %enum_val66 = load %Token, ptr %Token_ctor64, align 1
  ret %Token %enum_val66

match_test_10:                                    ; preds = %match_test_9
  %str_slot67 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot67, align 8
  %str_slot68 = alloca %NomString, align 8
  store %NomString { ptr @str.15, i64 4 }, ptr %str_slot68, align 8
  %match_str_eq69 = call i32 @nom_string_eq(ptr %str_slot67, ptr %str_slot68)
  %match_str_bool70 = icmp ne i32 %match_str_eq69, 0
  br i1 %match_str_bool70, label %match_arm_10, label %match_test_11

match_arm_10:                                     ; preds = %match_test_10
  %Token_ctor71 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor71, align 1
  %tag_ptr72 = getelementptr inbounds %Token, ptr %Token_ctor71, i32 0, i32 0
  store i8 10, ptr %tag_ptr72, align 1
  %enum_val73 = load %Token, ptr %Token_ctor71, align 1
  ret %Token %enum_val73

match_test_11:                                    ; preds = %match_test_10
  %str_slot74 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot74, align 8
  %str_slot75 = alloca %NomString, align 8
  store %NomString { ptr @str.16, i64 7 }, ptr %str_slot75, align 8
  %match_str_eq76 = call i32 @nom_string_eq(ptr %str_slot74, ptr %str_slot75)
  %match_str_bool77 = icmp ne i32 %match_str_eq76, 0
  br i1 %match_str_bool77, label %match_arm_11, label %match_test_12

match_arm_11:                                     ; preds = %match_test_11
  %Token_ctor78 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor78, align 1
  %tag_ptr79 = getelementptr inbounds %Token, ptr %Token_ctor78, i32 0, i32 0
  store i8 11, ptr %tag_ptr79, align 1
  %enum_val80 = load %Token, ptr %Token_ctor78, align 1
  ret %Token %enum_val80

match_test_12:                                    ; preds = %match_test_11
  %str_slot81 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot81, align 8
  %str_slot82 = alloca %NomString, align 8
  store %NomString { ptr @str.17, i64 7 }, ptr %str_slot82, align 8
  %match_str_eq83 = call i32 @nom_string_eq(ptr %str_slot81, ptr %str_slot82)
  %match_str_bool84 = icmp ne i32 %match_str_eq83, 0
  br i1 %match_str_bool84, label %match_arm_12, label %match_test_13

match_arm_12:                                     ; preds = %match_test_12
  %Token_ctor85 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor85, align 1
  %tag_ptr86 = getelementptr inbounds %Token, ptr %Token_ctor85, i32 0, i32 0
  store i8 12, ptr %tag_ptr86, align 1
  %enum_val87 = load %Token, ptr %Token_ctor85, align 1
  ret %Token %enum_val87

match_test_13:                                    ; preds = %match_test_12
  %str_slot88 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot88, align 8
  %str_slot89 = alloca %NomString, align 8
  store %NomString { ptr @str.18, i64 5 }, ptr %str_slot89, align 8
  %match_str_eq90 = call i32 @nom_string_eq(ptr %str_slot88, ptr %str_slot89)
  %match_str_bool91 = icmp ne i32 %match_str_eq90, 0
  br i1 %match_str_bool91, label %match_arm_13, label %match_test_14

match_arm_13:                                     ; preds = %match_test_13
  %Token_ctor92 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor92, align 1
  %tag_ptr93 = getelementptr inbounds %Token, ptr %Token_ctor92, i32 0, i32 0
  store i8 13, ptr %tag_ptr93, align 1
  %enum_val94 = load %Token, ptr %Token_ctor92, align 1
  ret %Token %enum_val94

match_test_14:                                    ; preds = %match_test_13
  %str_slot95 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot95, align 8
  %str_slot96 = alloca %NomString, align 8
  store %NomString { ptr @str.19, i64 4 }, ptr %str_slot96, align 8
  %match_str_eq97 = call i32 @nom_string_eq(ptr %str_slot95, ptr %str_slot96)
  %match_str_bool98 = icmp ne i32 %match_str_eq97, 0
  br i1 %match_str_bool98, label %match_arm_14, label %match_test_15

match_arm_14:                                     ; preds = %match_test_14
  %Token_ctor99 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor99, align 1
  %tag_ptr100 = getelementptr inbounds %Token, ptr %Token_ctor99, i32 0, i32 0
  store i8 14, ptr %tag_ptr100, align 1
  %enum_val101 = load %Token, ptr %Token_ctor99, align 1
  ret %Token %enum_val101

match_test_15:                                    ; preds = %match_test_14
  %str_slot102 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot102, align 8
  %str_slot103 = alloca %NomString, align 8
  store %NomString { ptr @str.20, i64 8 }, ptr %str_slot103, align 8
  %match_str_eq104 = call i32 @nom_string_eq(ptr %str_slot102, ptr %str_slot103)
  %match_str_bool105 = icmp ne i32 %match_str_eq104, 0
  br i1 %match_str_bool105, label %match_arm_15, label %match_test_16

match_arm_15:                                     ; preds = %match_test_15
  %Token_ctor106 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor106, align 1
  %tag_ptr107 = getelementptr inbounds %Token, ptr %Token_ctor106, i32 0, i32 0
  store i8 15, ptr %tag_ptr107, align 1
  %enum_val108 = load %Token, ptr %Token_ctor106, align 1
  ret %Token %enum_val108

match_test_16:                                    ; preds = %match_test_15
  %str_slot109 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot109, align 8
  %str_slot110 = alloca %NomString, align 8
  store %NomString { ptr @str.21, i64 6 }, ptr %str_slot110, align 8
  %match_str_eq111 = call i32 @nom_string_eq(ptr %str_slot109, ptr %str_slot110)
  %match_str_bool112 = icmp ne i32 %match_str_eq111, 0
  br i1 %match_str_bool112, label %match_arm_16, label %match_test_17

match_arm_16:                                     ; preds = %match_test_16
  %Token_ctor113 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor113, align 1
  %tag_ptr114 = getelementptr inbounds %Token, ptr %Token_ctor113, i32 0, i32 0
  store i8 16, ptr %tag_ptr114, align 1
  %enum_val115 = load %Token, ptr %Token_ctor113, align 1
  ret %Token %enum_val115

match_test_17:                                    ; preds = %match_test_16
  %str_slot116 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot116, align 8
  %str_slot117 = alloca %NomString, align 8
  store %NomString { ptr @str.22, i64 6 }, ptr %str_slot117, align 8
  %match_str_eq118 = call i32 @nom_string_eq(ptr %str_slot116, ptr %str_slot117)
  %match_str_bool119 = icmp ne i32 %match_str_eq118, 0
  br i1 %match_str_bool119, label %match_arm_17, label %match_test_18

match_arm_17:                                     ; preds = %match_test_17
  %Token_ctor120 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor120, align 1
  %tag_ptr121 = getelementptr inbounds %Token, ptr %Token_ctor120, i32 0, i32 0
  store i8 17, ptr %tag_ptr121, align 1
  %enum_val122 = load %Token, ptr %Token_ctor120, align 1
  ret %Token %enum_val122

match_test_18:                                    ; preds = %match_test_17
  %str_slot123 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot123, align 8
  %str_slot124 = alloca %NomString, align 8
  store %NomString { ptr @str.23, i64 7 }, ptr %str_slot124, align 8
  %match_str_eq125 = call i32 @nom_string_eq(ptr %str_slot123, ptr %str_slot124)
  %match_str_bool126 = icmp ne i32 %match_str_eq125, 0
  br i1 %match_str_bool126, label %match_arm_18, label %match_test_19

match_arm_18:                                     ; preds = %match_test_18
  %Token_ctor127 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor127, align 1
  %tag_ptr128 = getelementptr inbounds %Token, ptr %Token_ctor127, i32 0, i32 0
  store i8 18, ptr %tag_ptr128, align 1
  %enum_val129 = load %Token, ptr %Token_ctor127, align 1
  ret %Token %enum_val129

match_test_19:                                    ; preds = %match_test_18
  %str_slot130 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot130, align 8
  %str_slot131 = alloca %NomString, align 8
  store %NomString { ptr @str.24, i64 5 }, ptr %str_slot131, align 8
  %match_str_eq132 = call i32 @nom_string_eq(ptr %str_slot130, ptr %str_slot131)
  %match_str_bool133 = icmp ne i32 %match_str_eq132, 0
  br i1 %match_str_bool133, label %match_arm_19, label %match_test_20

match_arm_19:                                     ; preds = %match_test_19
  %Token_ctor134 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor134, align 1
  %tag_ptr135 = getelementptr inbounds %Token, ptr %Token_ctor134, i32 0, i32 0
  store i8 19, ptr %tag_ptr135, align 1
  %enum_val136 = load %Token, ptr %Token_ctor134, align 1
  ret %Token %enum_val136

match_test_20:                                    ; preds = %match_test_19
  %str_slot137 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot137, align 8
  %str_slot138 = alloca %NomString, align 8
  store %NomString { ptr @str.25, i64 4 }, ptr %str_slot138, align 8
  %match_str_eq139 = call i32 @nom_string_eq(ptr %str_slot137, ptr %str_slot138)
  %match_str_bool140 = icmp ne i32 %match_str_eq139, 0
  br i1 %match_str_bool140, label %match_arm_20, label %match_test_21

match_arm_20:                                     ; preds = %match_test_20
  %Token_ctor141 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor141, align 1
  %tag_ptr142 = getelementptr inbounds %Token, ptr %Token_ctor141, i32 0, i32 0
  store i8 20, ptr %tag_ptr142, align 1
  %enum_val143 = load %Token, ptr %Token_ctor141, align 1
  ret %Token %enum_val143

match_test_21:                                    ; preds = %match_test_20
  %str_slot144 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot144, align 8
  %str_slot145 = alloca %NomString, align 8
  store %NomString { ptr @str.26, i64 4 }, ptr %str_slot145, align 8
  %match_str_eq146 = call i32 @nom_string_eq(ptr %str_slot144, ptr %str_slot145)
  %match_str_bool147 = icmp ne i32 %match_str_eq146, 0
  br i1 %match_str_bool147, label %match_arm_21, label %match_test_22

match_arm_21:                                     ; preds = %match_test_21
  %Token_ctor148 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor148, align 1
  %tag_ptr149 = getelementptr inbounds %Token, ptr %Token_ctor148, i32 0, i32 0
  store i8 21, ptr %tag_ptr149, align 1
  %enum_val150 = load %Token, ptr %Token_ctor148, align 1
  ret %Token %enum_val150

match_test_22:                                    ; preds = %match_test_21
  %str_slot151 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot151, align 8
  %str_slot152 = alloca %NomString, align 8
  store %NomString { ptr @str.27, i64 3 }, ptr %str_slot152, align 8
  %match_str_eq153 = call i32 @nom_string_eq(ptr %str_slot151, ptr %str_slot152)
  %match_str_bool154 = icmp ne i32 %match_str_eq153, 0
  br i1 %match_str_bool154, label %match_arm_22, label %match_test_23

match_arm_22:                                     ; preds = %match_test_22
  %Token_ctor155 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor155, align 1
  %tag_ptr156 = getelementptr inbounds %Token, ptr %Token_ctor155, i32 0, i32 0
  store i8 22, ptr %tag_ptr156, align 1
  %enum_val157 = load %Token, ptr %Token_ctor155, align 1
  ret %Token %enum_val157

match_test_23:                                    ; preds = %match_test_22
  %str_slot158 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot158, align 8
  %str_slot159 = alloca %NomString, align 8
  store %NomString { ptr @str.28, i64 2 }, ptr %str_slot159, align 8
  %match_str_eq160 = call i32 @nom_string_eq(ptr %str_slot158, ptr %str_slot159)
  %match_str_bool161 = icmp ne i32 %match_str_eq160, 0
  br i1 %match_str_bool161, label %match_arm_23, label %match_test_24

match_arm_23:                                     ; preds = %match_test_23
  %Token_ctor162 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor162, align 1
  %tag_ptr163 = getelementptr inbounds %Token, ptr %Token_ctor162, i32 0, i32 0
  store i8 23, ptr %tag_ptr163, align 1
  %enum_val164 = load %Token, ptr %Token_ctor162, align 1
  ret %Token %enum_val164

match_test_24:                                    ; preds = %match_test_23
  %str_slot165 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot165, align 8
  %str_slot166 = alloca %NomString, align 8
  store %NomString { ptr @str.29, i64 8 }, ptr %str_slot166, align 8
  %match_str_eq167 = call i32 @nom_string_eq(ptr %str_slot165, ptr %str_slot166)
  %match_str_bool168 = icmp ne i32 %match_str_eq167, 0
  br i1 %match_str_bool168, label %match_arm_24, label %match_test_25

match_arm_24:                                     ; preds = %match_test_24
  %Token_ctor169 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor169, align 1
  %tag_ptr170 = getelementptr inbounds %Token, ptr %Token_ctor169, i32 0, i32 0
  store i8 24, ptr %tag_ptr170, align 1
  %enum_val171 = load %Token, ptr %Token_ctor169, align 1
  ret %Token %enum_val171

match_test_25:                                    ; preds = %match_test_24
  %str_slot172 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot172, align 8
  %str_slot173 = alloca %NomString, align 8
  store %NomString { ptr @str.30, i64 9 }, ptr %str_slot173, align 8
  %match_str_eq174 = call i32 @nom_string_eq(ptr %str_slot172, ptr %str_slot173)
  %match_str_bool175 = icmp ne i32 %match_str_eq174, 0
  br i1 %match_str_bool175, label %match_arm_25, label %match_test_26

match_arm_25:                                     ; preds = %match_test_25
  %Token_ctor176 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor176, align 1
  %tag_ptr177 = getelementptr inbounds %Token, ptr %Token_ctor176, i32 0, i32 0
  store i8 25, ptr %tag_ptr177, align 1
  %enum_val178 = load %Token, ptr %Token_ctor176, align 1
  ret %Token %enum_val178

match_test_26:                                    ; preds = %match_test_25
  %str_slot179 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot179, align 8
  %str_slot180 = alloca %NomString, align 8
  store %NomString { ptr @str.31, i64 4 }, ptr %str_slot180, align 8
  %match_str_eq181 = call i32 @nom_string_eq(ptr %str_slot179, ptr %str_slot180)
  %match_str_bool182 = icmp ne i32 %match_str_eq181, 0
  br i1 %match_str_bool182, label %match_arm_26, label %match_test_27

match_arm_26:                                     ; preds = %match_test_26
  %Token_ctor183 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor183, align 1
  %tag_ptr184 = getelementptr inbounds %Token, ptr %Token_ctor183, i32 0, i32 0
  store i8 26, ptr %tag_ptr184, align 1
  %enum_val185 = load %Token, ptr %Token_ctor183, align 1
  ret %Token %enum_val185

match_test_27:                                    ; preds = %match_test_26
  %str_slot186 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot186, align 8
  %str_slot187 = alloca %NomString, align 8
  store %NomString { ptr @str.32, i64 4 }, ptr %str_slot187, align 8
  %match_str_eq188 = call i32 @nom_string_eq(ptr %str_slot186, ptr %str_slot187)
  %match_str_bool189 = icmp ne i32 %match_str_eq188, 0
  br i1 %match_str_bool189, label %match_arm_27, label %match_test_28

match_arm_27:                                     ; preds = %match_test_27
  %Token_ctor190 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor190, align 1
  %tag_ptr191 = getelementptr inbounds %Token, ptr %Token_ctor190, i32 0, i32 0
  store i8 27, ptr %tag_ptr191, align 1
  %enum_val192 = load %Token, ptr %Token_ctor190, align 1
  ret %Token %enum_val192

match_test_28:                                    ; preds = %match_test_27
  %str_slot193 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot193, align 8
  %str_slot194 = alloca %NomString, align 8
  store %NomString { ptr @str.33, i64 5 }, ptr %str_slot194, align 8
  %match_str_eq195 = call i32 @nom_string_eq(ptr %str_slot193, ptr %str_slot194)
  %match_str_bool196 = icmp ne i32 %match_str_eq195, 0
  br i1 %match_str_bool196, label %match_arm_28, label %match_test_29

match_arm_28:                                     ; preds = %match_test_28
  %Token_ctor197 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor197, align 1
  %tag_ptr198 = getelementptr inbounds %Token, ptr %Token_ctor197, i32 0, i32 0
  store i8 28, ptr %tag_ptr198, align 1
  %enum_val199 = load %Token, ptr %Token_ctor197, align 1
  ret %Token %enum_val199

match_test_29:                                    ; preds = %match_test_28
  %str_slot200 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot200, align 8
  %str_slot201 = alloca %NomString, align 8
  store %NomString { ptr @str.34, i64 10 }, ptr %str_slot201, align 8
  %match_str_eq202 = call i32 @nom_string_eq(ptr %str_slot200, ptr %str_slot201)
  %match_str_bool203 = icmp ne i32 %match_str_eq202, 0
  br i1 %match_str_bool203, label %match_arm_29, label %match_test_30

match_arm_29:                                     ; preds = %match_test_29
  %Token_ctor204 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor204, align 1
  %tag_ptr205 = getelementptr inbounds %Token, ptr %Token_ctor204, i32 0, i32 0
  store i8 29, ptr %tag_ptr205, align 1
  %enum_val206 = load %Token, ptr %Token_ctor204, align 1
  ret %Token %enum_val206

match_test_30:                                    ; preds = %match_test_29
  %str_slot207 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot207, align 8
  %str_slot208 = alloca %NomString, align 8
  store %NomString { ptr @str.35, i64 10 }, ptr %str_slot208, align 8
  %match_str_eq209 = call i32 @nom_string_eq(ptr %str_slot207, ptr %str_slot208)
  %match_str_bool210 = icmp ne i32 %match_str_eq209, 0
  br i1 %match_str_bool210, label %match_arm_30, label %match_test_31

match_arm_30:                                     ; preds = %match_test_30
  %Token_ctor211 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor211, align 1
  %tag_ptr212 = getelementptr inbounds %Token, ptr %Token_ctor211, i32 0, i32 0
  store i8 30, ptr %tag_ptr212, align 1
  %enum_val213 = load %Token, ptr %Token_ctor211, align 1
  ret %Token %enum_val213

match_test_31:                                    ; preds = %match_test_30
  %str_slot214 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot214, align 8
  %str_slot215 = alloca %NomString, align 8
  store %NomString { ptr @str.36, i64 9 }, ptr %str_slot215, align 8
  %match_str_eq216 = call i32 @nom_string_eq(ptr %str_slot214, ptr %str_slot215)
  %match_str_bool217 = icmp ne i32 %match_str_eq216, 0
  br i1 %match_str_bool217, label %match_arm_31, label %match_test_32

match_arm_31:                                     ; preds = %match_test_31
  %Token_ctor218 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor218, align 1
  %tag_ptr219 = getelementptr inbounds %Token, ptr %Token_ctor218, i32 0, i32 0
  store i8 31, ptr %tag_ptr219, align 1
  %enum_val220 = load %Token, ptr %Token_ctor218, align 1
  ret %Token %enum_val220

match_test_32:                                    ; preds = %match_test_31
  %str_slot221 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot221, align 8
  %str_slot222 = alloca %NomString, align 8
  store %NomString { ptr @str.37, i64 7 }, ptr %str_slot222, align 8
  %match_str_eq223 = call i32 @nom_string_eq(ptr %str_slot221, ptr %str_slot222)
  %match_str_bool224 = icmp ne i32 %match_str_eq223, 0
  br i1 %match_str_bool224, label %match_arm_32, label %match_test_33

match_arm_32:                                     ; preds = %match_test_32
  %Token_ctor225 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor225, align 1
  %tag_ptr226 = getelementptr inbounds %Token, ptr %Token_ctor225, i32 0, i32 0
  store i8 32, ptr %tag_ptr226, align 1
  %enum_val227 = load %Token, ptr %Token_ctor225, align 1
  ret %Token %enum_val227

match_test_33:                                    ; preds = %match_test_32
  %str_slot228 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot228, align 8
  %str_slot229 = alloca %NomString, align 8
  store %NomString { ptr @str.38, i64 5 }, ptr %str_slot229, align 8
  %match_str_eq230 = call i32 @nom_string_eq(ptr %str_slot228, ptr %str_slot229)
  %match_str_bool231 = icmp ne i32 %match_str_eq230, 0
  br i1 %match_str_bool231, label %match_arm_33, label %match_test_34

match_arm_33:                                     ; preds = %match_test_33
  %Token_ctor232 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor232, align 1
  %tag_ptr233 = getelementptr inbounds %Token, ptr %Token_ctor232, i32 0, i32 0
  store i8 33, ptr %tag_ptr233, align 1
  %enum_val234 = load %Token, ptr %Token_ctor232, align 1
  ret %Token %enum_val234

match_test_34:                                    ; preds = %match_test_33
  %str_slot235 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot235, align 8
  %str_slot236 = alloca %NomString, align 8
  store %NomString { ptr @str.39, i64 8 }, ptr %str_slot236, align 8
  %match_str_eq237 = call i32 @nom_string_eq(ptr %str_slot235, ptr %str_slot236)
  %match_str_bool238 = icmp ne i32 %match_str_eq237, 0
  br i1 %match_str_bool238, label %match_arm_34, label %match_test_35

match_arm_34:                                     ; preds = %match_test_34
  %Token_ctor239 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor239, align 1
  %tag_ptr240 = getelementptr inbounds %Token, ptr %Token_ctor239, i32 0, i32 0
  store i8 34, ptr %tag_ptr240, align 1
  %enum_val241 = load %Token, ptr %Token_ctor239, align 1
  ret %Token %enum_val241

match_test_35:                                    ; preds = %match_test_34
  %str_slot242 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot242, align 8
  %str_slot243 = alloca %NomString, align 8
  store %NomString { ptr @str.40, i64 5 }, ptr %str_slot243, align 8
  %match_str_eq244 = call i32 @nom_string_eq(ptr %str_slot242, ptr %str_slot243)
  %match_str_bool245 = icmp ne i32 %match_str_eq244, 0
  br i1 %match_str_bool245, label %match_arm_35, label %match_test_36

match_arm_35:                                     ; preds = %match_test_35
  %Token_ctor246 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor246, align 1
  %tag_ptr247 = getelementptr inbounds %Token, ptr %Token_ctor246, i32 0, i32 0
  store i8 35, ptr %tag_ptr247, align 1
  %enum_val248 = load %Token, ptr %Token_ctor246, align 1
  ret %Token %enum_val248

match_test_36:                                    ; preds = %match_test_35
  %str_slot249 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot249, align 8
  %str_slot250 = alloca %NomString, align 8
  store %NomString { ptr @str.41, i64 3 }, ptr %str_slot250, align 8
  %match_str_eq251 = call i32 @nom_string_eq(ptr %str_slot249, ptr %str_slot250)
  %match_str_bool252 = icmp ne i32 %match_str_eq251, 0
  br i1 %match_str_bool252, label %match_arm_36, label %match_test_37

match_arm_36:                                     ; preds = %match_test_36
  %Token_ctor253 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor253, align 1
  %tag_ptr254 = getelementptr inbounds %Token, ptr %Token_ctor253, i32 0, i32 0
  store i8 36, ptr %tag_ptr254, align 1
  %enum_val255 = load %Token, ptr %Token_ctor253, align 1
  ret %Token %enum_val255

match_test_37:                                    ; preds = %match_test_36
  %str_slot256 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot256, align 8
  %str_slot257 = alloca %NomString, align 8
  store %NomString { ptr @str.42, i64 3 }, ptr %str_slot257, align 8
  %match_str_eq258 = call i32 @nom_string_eq(ptr %str_slot256, ptr %str_slot257)
  %match_str_bool259 = icmp ne i32 %match_str_eq258, 0
  br i1 %match_str_bool259, label %match_arm_37, label %match_test_38

match_arm_37:                                     ; preds = %match_test_37
  %Token_ctor260 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor260, align 1
  %tag_ptr261 = getelementptr inbounds %Token, ptr %Token_ctor260, i32 0, i32 0
  store i8 37, ptr %tag_ptr261, align 1
  %enum_val262 = load %Token, ptr %Token_ctor260, align 1
  ret %Token %enum_val262

match_test_38:                                    ; preds = %match_test_37
  %str_slot263 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot263, align 8
  %str_slot264 = alloca %NomString, align 8
  store %NomString { ptr @str.43, i64 2 }, ptr %str_slot264, align 8
  %match_str_eq265 = call i32 @nom_string_eq(ptr %str_slot263, ptr %str_slot264)
  %match_str_bool266 = icmp ne i32 %match_str_eq265, 0
  br i1 %match_str_bool266, label %match_arm_38, label %match_test_39

match_arm_38:                                     ; preds = %match_test_38
  %Token_ctor267 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor267, align 1
  %tag_ptr268 = getelementptr inbounds %Token, ptr %Token_ctor267, i32 0, i32 0
  store i8 38, ptr %tag_ptr268, align 1
  %enum_val269 = load %Token, ptr %Token_ctor267, align 1
  ret %Token %enum_val269

match_test_39:                                    ; preds = %match_test_38
  %str_slot270 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot270, align 8
  %str_slot271 = alloca %NomString, align 8
  store %NomString { ptr @str.44, i64 4 }, ptr %str_slot271, align 8
  %match_str_eq272 = call i32 @nom_string_eq(ptr %str_slot270, ptr %str_slot271)
  %match_str_bool273 = icmp ne i32 %match_str_eq272, 0
  br i1 %match_str_bool273, label %match_arm_39, label %match_test_40

match_arm_39:                                     ; preds = %match_test_39
  %Token_ctor274 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor274, align 1
  %tag_ptr275 = getelementptr inbounds %Token, ptr %Token_ctor274, i32 0, i32 0
  store i8 39, ptr %tag_ptr275, align 1
  %enum_val276 = load %Token, ptr %Token_ctor274, align 1
  ret %Token %enum_val276

match_test_40:                                    ; preds = %match_test_39
  %str_slot277 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot277, align 8
  %str_slot278 = alloca %NomString, align 8
  store %NomString { ptr @str.45, i64 3 }, ptr %str_slot278, align 8
  %match_str_eq279 = call i32 @nom_string_eq(ptr %str_slot277, ptr %str_slot278)
  %match_str_bool280 = icmp ne i32 %match_str_eq279, 0
  br i1 %match_str_bool280, label %match_arm_40, label %match_test_41

match_arm_40:                                     ; preds = %match_test_40
  %Token_ctor281 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor281, align 1
  %tag_ptr282 = getelementptr inbounds %Token, ptr %Token_ctor281, i32 0, i32 0
  store i8 40, ptr %tag_ptr282, align 1
  %enum_val283 = load %Token, ptr %Token_ctor281, align 1
  ret %Token %enum_val283

match_test_41:                                    ; preds = %match_test_40
  %str_slot284 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot284, align 8
  %str_slot285 = alloca %NomString, align 8
  store %NomString { ptr @str.46, i64 5 }, ptr %str_slot285, align 8
  %match_str_eq286 = call i32 @nom_string_eq(ptr %str_slot284, ptr %str_slot285)
  %match_str_bool287 = icmp ne i32 %match_str_eq286, 0
  br i1 %match_str_bool287, label %match_arm_41, label %match_test_42

match_arm_41:                                     ; preds = %match_test_41
  %Token_ctor288 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor288, align 1
  %tag_ptr289 = getelementptr inbounds %Token, ptr %Token_ctor288, i32 0, i32 0
  store i8 41, ptr %tag_ptr289, align 1
  %enum_val290 = load %Token, ptr %Token_ctor288, align 1
  ret %Token %enum_val290

match_test_42:                                    ; preds = %match_test_41
  %str_slot291 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot291, align 8
  %str_slot292 = alloca %NomString, align 8
  store %NomString { ptr @str.47, i64 4 }, ptr %str_slot292, align 8
  %match_str_eq293 = call i32 @nom_string_eq(ptr %str_slot291, ptr %str_slot292)
  %match_str_bool294 = icmp ne i32 %match_str_eq293, 0
  br i1 %match_str_bool294, label %match_arm_42, label %match_test_43

match_arm_42:                                     ; preds = %match_test_42
  %Token_ctor295 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor295, align 1
  %tag_ptr296 = getelementptr inbounds %Token, ptr %Token_ctor295, i32 0, i32 0
  store i8 42, ptr %tag_ptr296, align 1
  %enum_val297 = load %Token, ptr %Token_ctor295, align 1
  ret %Token %enum_val297

match_test_43:                                    ; preds = %match_test_42
  %str_slot298 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot298, align 8
  %str_slot299 = alloca %NomString, align 8
  store %NomString { ptr @str.48, i64 5 }, ptr %str_slot299, align 8
  %match_str_eq300 = call i32 @nom_string_eq(ptr %str_slot298, ptr %str_slot299)
  %match_str_bool301 = icmp ne i32 %match_str_eq300, 0
  br i1 %match_str_bool301, label %match_arm_43, label %match_test_44

match_arm_43:                                     ; preds = %match_test_43
  %Token_ctor302 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor302, align 1
  %tag_ptr303 = getelementptr inbounds %Token, ptr %Token_ctor302, i32 0, i32 0
  store i8 43, ptr %tag_ptr303, align 1
  %enum_val304 = load %Token, ptr %Token_ctor302, align 1
  ret %Token %enum_val304

match_test_44:                                    ; preds = %match_test_43
  %str_slot305 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot305, align 8
  %str_slot306 = alloca %NomString, align 8
  store %NomString { ptr @str.49, i64 6 }, ptr %str_slot306, align 8
  %match_str_eq307 = call i32 @nom_string_eq(ptr %str_slot305, ptr %str_slot306)
  %match_str_bool308 = icmp ne i32 %match_str_eq307, 0
  br i1 %match_str_bool308, label %match_arm_44, label %match_test_45

match_arm_44:                                     ; preds = %match_test_44
  %Token_ctor309 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor309, align 1
  %tag_ptr310 = getelementptr inbounds %Token, ptr %Token_ctor309, i32 0, i32 0
  store i8 44, ptr %tag_ptr310, align 1
  %enum_val311 = load %Token, ptr %Token_ctor309, align 1
  ret %Token %enum_val311

match_test_45:                                    ; preds = %match_test_44
  %str_slot312 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot312, align 8
  %str_slot313 = alloca %NomString, align 8
  store %NomString { ptr @str.50, i64 5 }, ptr %str_slot313, align 8
  %match_str_eq314 = call i32 @nom_string_eq(ptr %str_slot312, ptr %str_slot313)
  %match_str_bool315 = icmp ne i32 %match_str_eq314, 0
  br i1 %match_str_bool315, label %match_arm_45, label %match_test_46

match_arm_45:                                     ; preds = %match_test_45
  %Token_ctor316 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor316, align 1
  %tag_ptr317 = getelementptr inbounds %Token, ptr %Token_ctor316, i32 0, i32 0
  store i8 45, ptr %tag_ptr317, align 1
  %enum_val318 = load %Token, ptr %Token_ctor316, align 1
  ret %Token %enum_val318

match_test_46:                                    ; preds = %match_test_45
  %str_slot319 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot319, align 8
  %str_slot320 = alloca %NomString, align 8
  store %NomString { ptr @str.51, i64 8 }, ptr %str_slot320, align 8
  %match_str_eq321 = call i32 @nom_string_eq(ptr %str_slot319, ptr %str_slot320)
  %match_str_bool322 = icmp ne i32 %match_str_eq321, 0
  br i1 %match_str_bool322, label %match_arm_46, label %match_test_47

match_arm_46:                                     ; preds = %match_test_46
  %Token_ctor323 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor323, align 1
  %tag_ptr324 = getelementptr inbounds %Token, ptr %Token_ctor323, i32 0, i32 0
  store i8 46, ptr %tag_ptr324, align 1
  %enum_val325 = load %Token, ptr %Token_ctor323, align 1
  ret %Token %enum_val325

match_test_47:                                    ; preds = %match_test_46
  %str_slot326 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot326, align 8
  %str_slot327 = alloca %NomString, align 8
  store %NomString { ptr @str.52, i64 2 }, ptr %str_slot327, align 8
  %match_str_eq328 = call i32 @nom_string_eq(ptr %str_slot326, ptr %str_slot327)
  %match_str_bool329 = icmp ne i32 %match_str_eq328, 0
  br i1 %match_str_bool329, label %match_arm_47, label %match_test_48

match_arm_47:                                     ; preds = %match_test_47
  %Token_ctor330 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor330, align 1
  %tag_ptr331 = getelementptr inbounds %Token, ptr %Token_ctor330, i32 0, i32 0
  store i8 47, ptr %tag_ptr331, align 1
  %enum_val332 = load %Token, ptr %Token_ctor330, align 1
  ret %Token %enum_val332

match_test_48:                                    ; preds = %match_test_47
  %str_slot333 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot333, align 8
  %str_slot334 = alloca %NomString, align 8
  store %NomString { ptr @str.53, i64 4 }, ptr %str_slot334, align 8
  %match_str_eq335 = call i32 @nom_string_eq(ptr %str_slot333, ptr %str_slot334)
  %match_str_bool336 = icmp ne i32 %match_str_eq335, 0
  br i1 %match_str_bool336, label %match_arm_48, label %match_test_49

match_arm_48:                                     ; preds = %match_test_48
  %Token_ctor337 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor337, align 1
  %tag_ptr338 = getelementptr inbounds %Token, ptr %Token_ctor337, i32 0, i32 0
  store i8 48, ptr %tag_ptr338, align 1
  %enum_val339 = load %Token, ptr %Token_ctor337, align 1
  ret %Token %enum_val339

match_test_49:                                    ; preds = %match_test_48
  %str_slot340 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot340, align 8
  %str_slot341 = alloca %NomString, align 8
  store %NomString { ptr @str.54, i64 6 }, ptr %str_slot341, align 8
  %match_str_eq342 = call i32 @nom_string_eq(ptr %str_slot340, ptr %str_slot341)
  %match_str_bool343 = icmp ne i32 %match_str_eq342, 0
  br i1 %match_str_bool343, label %match_arm_49, label %match_test_50

match_arm_49:                                     ; preds = %match_test_49
  %Token_ctor344 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor344, align 1
  %tag_ptr345 = getelementptr inbounds %Token, ptr %Token_ctor344, i32 0, i32 0
  store i8 49, ptr %tag_ptr345, align 1
  %enum_val346 = load %Token, ptr %Token_ctor344, align 1
  ret %Token %enum_val346

match_test_50:                                    ; preds = %match_test_49
  %str_slot347 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot347, align 8
  %str_slot348 = alloca %NomString, align 8
  store %NomString { ptr @str.55, i64 4 }, ptr %str_slot348, align 8
  %match_str_eq349 = call i32 @nom_string_eq(ptr %str_slot347, ptr %str_slot348)
  %match_str_bool350 = icmp ne i32 %match_str_eq349, 0
  br i1 %match_str_bool350, label %match_arm_50, label %match_test_51

match_arm_50:                                     ; preds = %match_test_50
  %Token_ctor351 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor351, align 1
  %tag_ptr352 = getelementptr inbounds %Token, ptr %Token_ctor351, i32 0, i32 0
  store i8 50, ptr %tag_ptr352, align 1
  %enum_val353 = load %Token, ptr %Token_ctor351, align 1
  ret %Token %enum_val353

match_test_51:                                    ; preds = %match_test_50
  %str_slot354 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot354, align 8
  %str_slot355 = alloca %NomString, align 8
  store %NomString { ptr @str.56, i64 3 }, ptr %str_slot355, align 8
  %match_str_eq356 = call i32 @nom_string_eq(ptr %str_slot354, ptr %str_slot355)
  %match_str_bool357 = icmp ne i32 %match_str_eq356, 0
  br i1 %match_str_bool357, label %match_arm_51, label %match_test_52

match_arm_51:                                     ; preds = %match_test_51
  %Token_ctor358 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor358, align 1
  %tag_ptr359 = getelementptr inbounds %Token, ptr %Token_ctor358, i32 0, i32 0
  store i8 51, ptr %tag_ptr359, align 1
  %enum_val360 = load %Token, ptr %Token_ctor358, align 1
  ret %Token %enum_val360

match_test_52:                                    ; preds = %match_test_51
  %str_slot361 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot361, align 8
  %str_slot362 = alloca %NomString, align 8
  store %NomString { ptr @str.57, i64 3 }, ptr %str_slot362, align 8
  %match_str_eq363 = call i32 @nom_string_eq(ptr %str_slot361, ptr %str_slot362)
  %match_str_bool364 = icmp ne i32 %match_str_eq363, 0
  br i1 %match_str_bool364, label %match_arm_52, label %match_test_53

match_arm_52:                                     ; preds = %match_test_52
  %Token_ctor365 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor365, align 1
  %tag_ptr366 = getelementptr inbounds %Token, ptr %Token_ctor365, i32 0, i32 0
  store i8 52, ptr %tag_ptr366, align 1
  %enum_val367 = load %Token, ptr %Token_ctor365, align 1
  ret %Token %enum_val367

match_test_53:                                    ; preds = %match_test_52
  %str_slot368 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot368, align 8
  %str_slot369 = alloca %NomString, align 8
  store %NomString { ptr @str.58, i64 2 }, ptr %str_slot369, align 8
  %match_str_eq370 = call i32 @nom_string_eq(ptr %str_slot368, ptr %str_slot369)
  %match_str_bool371 = icmp ne i32 %match_str_eq370, 0
  br i1 %match_str_bool371, label %match_arm_53, label %match_test_54

match_arm_53:                                     ; preds = %match_test_53
  %Token_ctor372 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor372, align 1
  %tag_ptr373 = getelementptr inbounds %Token, ptr %Token_ctor372, i32 0, i32 0
  store i8 53, ptr %tag_ptr373, align 1
  %enum_val374 = load %Token, ptr %Token_ctor372, align 1
  ret %Token %enum_val374

match_test_54:                                    ; preds = %match_test_53
  %str_slot375 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot375, align 8
  %str_slot376 = alloca %NomString, align 8
  store %NomString { ptr @str.59, i64 2 }, ptr %str_slot376, align 8
  %match_str_eq377 = call i32 @nom_string_eq(ptr %str_slot375, ptr %str_slot376)
  %match_str_bool378 = icmp ne i32 %match_str_eq377, 0
  br i1 %match_str_bool378, label %match_arm_54, label %match_test_55

match_arm_54:                                     ; preds = %match_test_54
  %Token_ctor379 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor379, align 1
  %tag_ptr380 = getelementptr inbounds %Token, ptr %Token_ctor379, i32 0, i32 0
  store i8 54, ptr %tag_ptr380, align 1
  %enum_val381 = load %Token, ptr %Token_ctor379, align 1
  ret %Token %enum_val381

match_test_55:                                    ; preds = %match_test_54
  %str_slot382 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot382, align 8
  %str_slot383 = alloca %NomString, align 8
  store %NomString { ptr @str.60, i64 3 }, ptr %str_slot383, align 8
  %match_str_eq384 = call i32 @nom_string_eq(ptr %str_slot382, ptr %str_slot383)
  %match_str_bool385 = icmp ne i32 %match_str_eq384, 0
  br i1 %match_str_bool385, label %match_arm_55, label %match_test_56

match_arm_55:                                     ; preds = %match_test_55
  %Token_ctor386 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor386, align 1
  %tag_ptr387 = getelementptr inbounds %Token, ptr %Token_ctor386, i32 0, i32 0
  store i8 55, ptr %tag_ptr387, align 1
  %enum_val388 = load %Token, ptr %Token_ctor386, align 1
  ret %Token %enum_val388

match_test_56:                                    ; preds = %match_test_55
  %str_slot389 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot389, align 8
  %str_slot390 = alloca %NomString, align 8
  store %NomString { ptr @str.61, i64 5 }, ptr %str_slot390, align 8
  %match_str_eq391 = call i32 @nom_string_eq(ptr %str_slot389, ptr %str_slot390)
  %match_str_bool392 = icmp ne i32 %match_str_eq391, 0
  br i1 %match_str_bool392, label %match_arm_56, label %match_test_57

match_arm_56:                                     ; preds = %match_test_56
  %Token_ctor393 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor393, align 1
  %tag_ptr394 = getelementptr inbounds %Token, ptr %Token_ctor393, i32 0, i32 0
  store i8 56, ptr %tag_ptr394, align 1
  %enum_val395 = load %Token, ptr %Token_ctor393, align 1
  ret %Token %enum_val395

match_test_57:                                    ; preds = %match_test_56
  %str_slot396 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot396, align 8
  %str_slot397 = alloca %NomString, align 8
  store %NomString { ptr @str.62, i64 4 }, ptr %str_slot397, align 8
  %match_str_eq398 = call i32 @nom_string_eq(ptr %str_slot396, ptr %str_slot397)
  %match_str_bool399 = icmp ne i32 %match_str_eq398, 0
  br i1 %match_str_bool399, label %match_arm_57, label %match_test_58

match_arm_57:                                     ; preds = %match_test_57
  %Token_ctor400 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor400, align 1
  %tag_ptr401 = getelementptr inbounds %Token, ptr %Token_ctor400, i32 0, i32 0
  store i8 57, ptr %tag_ptr401, align 1
  %enum_val402 = load %Token, ptr %Token_ctor400, align 1
  ret %Token %enum_val402

match_test_58:                                    ; preds = %match_test_57
  %str_slot403 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot403, align 8
  %str_slot404 = alloca %NomString, align 8
  store %NomString { ptr @str.63, i64 4 }, ptr %str_slot404, align 8
  %match_str_eq405 = call i32 @nom_string_eq(ptr %str_slot403, ptr %str_slot404)
  %match_str_bool406 = icmp ne i32 %match_str_eq405, 0
  br i1 %match_str_bool406, label %match_arm_58, label %match_test_59

match_arm_58:                                     ; preds = %match_test_58
  %Token_ctor407 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor407, align 1
  %tag_ptr408 = getelementptr inbounds %Token, ptr %Token_ctor407, i32 0, i32 0
  store i8 58, ptr %tag_ptr408, align 1
  %enum_val409 = load %Token, ptr %Token_ctor407, align 1
  ret %Token %enum_val409

match_test_59:                                    ; preds = %match_test_58
  %str_slot410 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot410, align 8
  %str_slot411 = alloca %NomString, align 8
  store %NomString { ptr @str.64, i64 5 }, ptr %str_slot411, align 8
  %match_str_eq412 = call i32 @nom_string_eq(ptr %str_slot410, ptr %str_slot411)
  %match_str_bool413 = icmp ne i32 %match_str_eq412, 0
  br i1 %match_str_bool413, label %match_arm_59, label %match_test_60

match_arm_59:                                     ; preds = %match_test_59
  %Token_ctor414 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor414, align 1
  %tag_ptr415 = getelementptr inbounds %Token, ptr %Token_ctor414, i32 0, i32 0
  store i8 59, ptr %tag_ptr415, align 1
  %enum_val416 = load %Token, ptr %Token_ctor414, align 1
  ret %Token %enum_val416

match_test_60:                                    ; preds = %match_test_59
  %str_slot417 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot417, align 8
  %str_slot418 = alloca %NomString, align 8
  store %NomString { ptr @str.65, i64 5 }, ptr %str_slot418, align 8
  %match_str_eq419 = call i32 @nom_string_eq(ptr %str_slot417, ptr %str_slot418)
  %match_str_bool420 = icmp ne i32 %match_str_eq419, 0
  br i1 %match_str_bool420, label %match_arm_60, label %match_test_61

match_arm_60:                                     ; preds = %match_test_60
  %Token_ctor421 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor421, align 1
  %tag_ptr422 = getelementptr inbounds %Token, ptr %Token_ctor421, i32 0, i32 0
  store i8 60, ptr %tag_ptr422, align 1
  %enum_val423 = load %Token, ptr %Token_ctor421, align 1
  ret %Token %enum_val423

match_test_61:                                    ; preds = %match_test_60
  %str_slot424 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot424, align 8
  %str_slot425 = alloca %NomString, align 8
  store %NomString { ptr @str.66, i64 6 }, ptr %str_slot425, align 8
  %match_str_eq426 = call i32 @nom_string_eq(ptr %str_slot424, ptr %str_slot425)
  %match_str_bool427 = icmp ne i32 %match_str_eq426, 0
  br i1 %match_str_bool427, label %match_arm_61, label %match_test_62

match_arm_61:                                     ; preds = %match_test_61
  %Token_ctor428 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor428, align 1
  %tag_ptr429 = getelementptr inbounds %Token, ptr %Token_ctor428, i32 0, i32 0
  store i8 47, ptr %tag_ptr429, align 1
  %enum_val430 = load %Token, ptr %Token_ctor428, align 1
  ret %Token %enum_val430

match_test_62:                                    ; preds = %match_test_61
  %str_slot431 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot431, align 8
  %str_slot432 = alloca %NomString, align 8
  store %NomString { ptr @str.67, i64 4 }, ptr %str_slot432, align 8
  %match_str_eq433 = call i32 @nom_string_eq(ptr %str_slot431, ptr %str_slot432)
  %match_str_bool434 = icmp ne i32 %match_str_eq433, 0
  br i1 %match_str_bool434, label %match_arm_62, label %match_test_63

match_arm_62:                                     ; preds = %match_test_62
  %Token_ctor435 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor435, align 1
  %tag_ptr436 = getelementptr inbounds %Token, ptr %Token_ctor435, i32 0, i32 0
  store i8 49, ptr %tag_ptr436, align 1
  %enum_val437 = load %Token, ptr %Token_ctor435, align 1
  ret %Token %enum_val437

match_test_63:                                    ; preds = %match_test_62
  %str_slot438 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot438, align 8
  %str_slot439 = alloca %NomString, align 8
  store %NomString { ptr @str.68, i64 6 }, ptr %str_slot439, align 8
  %match_str_eq440 = call i32 @nom_string_eq(ptr %str_slot438, ptr %str_slot439)
  %match_str_bool441 = icmp ne i32 %match_str_eq440, 0
  br i1 %match_str_bool441, label %match_arm_63, label %match_test_64

match_arm_63:                                     ; preds = %match_test_63
  %Token_ctor442 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor442, align 1
  %tag_ptr443 = getelementptr inbounds %Token, ptr %Token_ctor442, i32 0, i32 0
  store i8 50, ptr %tag_ptr443, align 1
  %enum_val444 = load %Token, ptr %Token_ctor442, align 1
  ret %Token %enum_val444

match_test_64:                                    ; preds = %match_test_63
  %str_slot445 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot445, align 8
  %str_slot446 = alloca %NomString, align 8
  store %NomString { ptr @str.69, i64 4 }, ptr %str_slot446, align 8
  %match_str_eq447 = call i32 @nom_string_eq(ptr %str_slot445, ptr %str_slot446)
  %match_str_bool448 = icmp ne i32 %match_str_eq447, 0
  br i1 %match_str_bool448, label %match_arm_64, label %match_test_65

match_arm_64:                                     ; preds = %match_test_64
  %Token_ctor449 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor449, align 1
  %tag_ptr450 = getelementptr inbounds %Token, ptr %Token_ctor449, i32 0, i32 0
  store i8 36, ptr %tag_ptr450, align 1
  %enum_val451 = load %Token, ptr %Token_ctor449, align 1
  ret %Token %enum_val451

match_test_65:                                    ; preds = %match_test_64
  %str_slot452 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot452, align 8
  %str_slot453 = alloca %NomString, align 8
  store %NomString { ptr @str.70, i64 3 }, ptr %str_slot453, align 8
  %match_str_eq454 = call i32 @nom_string_eq(ptr %str_slot452, ptr %str_slot453)
  %match_str_bool455 = icmp ne i32 %match_str_eq454, 0
  br i1 %match_str_bool455, label %match_arm_65, label %match_test_66

match_arm_65:                                     ; preds = %match_test_65
  %Token_ctor456 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor456, align 1
  %tag_ptr457 = getelementptr inbounds %Token, ptr %Token_ctor456, i32 0, i32 0
  store i8 36, ptr %tag_ptr457, align 1
  %enum_val458 = load %Token, ptr %Token_ctor456, align 1
  ret %Token %enum_val458

match_test_66:                                    ; preds = %match_test_65
  %str_slot459 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot459, align 8
  %str_slot460 = alloca %NomString, align 8
  store %NomString { ptr @str.71, i64 8 }, ptr %str_slot460, align 8
  %match_str_eq461 = call i32 @nom_string_eq(ptr %str_slot459, ptr %str_slot460)
  %match_str_bool462 = icmp ne i32 %match_str_eq461, 0
  br i1 %match_str_bool462, label %match_arm_66, label %match_test_67

match_arm_66:                                     ; preds = %match_test_66
  %Token_ctor463 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor463, align 1
  %tag_ptr464 = getelementptr inbounds %Token, ptr %Token_ctor463, i32 0, i32 0
  store i8 37, ptr %tag_ptr464, align 1
  %enum_val465 = load %Token, ptr %Token_ctor463, align 1
  ret %Token %enum_val465

match_test_67:                                    ; preds = %match_test_66
  %str_slot466 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot466, align 8
  %str_slot467 = alloca %NomString, align 8
  store %NomString { ptr @str.72, i64 4 }, ptr %str_slot467, align 8
  %match_str_eq468 = call i32 @nom_string_eq(ptr %str_slot466, ptr %str_slot467)
  %match_str_bool469 = icmp ne i32 %match_str_eq468, 0
  br i1 %match_str_bool469, label %match_arm_67, label %match_test_68

match_arm_67:                                     ; preds = %match_test_67
  %Token_ctor470 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor470, align 1
  %tag_ptr471 = getelementptr inbounds %Token, ptr %Token_ctor470, i32 0, i32 0
  store i8 44, ptr %tag_ptr471, align 1
  %enum_val472 = load %Token, ptr %Token_ctor470, align 1
  ret %Token %enum_val472

match_test_68:                                    ; preds = %match_test_67
  %str_slot473 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot473, align 8
  %str_slot474 = alloca %NomString, align 8
  store %NomString { ptr @str.73, i64 7 }, ptr %str_slot474, align 8
  %match_str_eq475 = call i32 @nom_string_eq(ptr %str_slot473, ptr %str_slot474)
  %match_str_bool476 = icmp ne i32 %match_str_eq475, 0
  br i1 %match_str_bool476, label %match_arm_68, label %match_test_69

match_arm_68:                                     ; preds = %match_test_68
  %Token_ctor477 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor477, align 1
  %tag_ptr478 = getelementptr inbounds %Token, ptr %Token_ctor477, i32 0, i32 0
  store i8 44, ptr %tag_ptr478, align 1
  %enum_val479 = load %Token, ptr %Token_ctor477, align 1
  ret %Token %enum_val479

match_test_69:                                    ; preds = %match_test_68
  %str_slot480 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot480, align 8
  %str_slot481 = alloca %NomString, align 8
  store %NomString { ptr @str.74, i64 4 }, ptr %str_slot481, align 8
  %match_str_eq482 = call i32 @nom_string_eq(ptr %str_slot480, ptr %str_slot481)
  %match_str_bool483 = icmp ne i32 %match_str_eq482, 0
  br i1 %match_str_bool483, label %match_arm_69, label %match_test_70

match_arm_69:                                     ; preds = %match_test_69
  %Token_ctor484 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor484, align 1
  %tag_ptr485 = getelementptr inbounds %Token, ptr %Token_ctor484, i32 0, i32 0
  store i8 40, ptr %tag_ptr485, align 1
  %enum_val486 = load %Token, ptr %Token_ctor484, align 1
  ret %Token %enum_val486

match_test_70:                                    ; preds = %match_test_69
  %str_slot487 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot487, align 8
  %str_slot488 = alloca %NomString, align 8
  store %NomString { ptr @str.75, i64 6 }, ptr %str_slot488, align 8
  %match_str_eq489 = call i32 @nom_string_eq(ptr %str_slot487, ptr %str_slot488)
  %match_str_bool490 = icmp ne i32 %match_str_eq489, 0
  br i1 %match_str_bool490, label %match_arm_70, label %match_test_71

match_arm_70:                                     ; preds = %match_test_70
  %Token_ctor491 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor491, align 1
  %tag_ptr492 = getelementptr inbounds %Token, ptr %Token_ctor491, i32 0, i32 0
  store i8 41, ptr %tag_ptr492, align 1
  %enum_val493 = load %Token, ptr %Token_ctor491, align 1
  ret %Token %enum_val493

match_test_71:                                    ; preds = %match_test_70
  %str_slot494 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot494, align 8
  %str_slot495 = alloca %NomString, align 8
  store %NomString { ptr @str.76, i64 5 }, ptr %str_slot495, align 8
  %match_str_eq496 = call i32 @nom_string_eq(ptr %str_slot494, ptr %str_slot495)
  %match_str_bool497 = icmp ne i32 %match_str_eq496, 0
  br i1 %match_str_bool497, label %match_arm_71, label %match_test_72

match_arm_71:                                     ; preds = %match_test_71
  %Token_ctor498 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor498, align 1
  %tag_ptr499 = getelementptr inbounds %Token, ptr %Token_ctor498, i32 0, i32 0
  store i8 43, ptr %tag_ptr499, align 1
  %enum_val500 = load %Token, ptr %Token_ctor498, align 1
  ret %Token %enum_val500

match_test_72:                                    ; preds = %match_test_71
  %str_slot501 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot501, align 8
  %str_slot502 = alloca %NomString, align 8
  store %NomString { ptr @str.77, i64 5 }, ptr %str_slot502, align 8
  %match_str_eq503 = call i32 @nom_string_eq(ptr %str_slot501, ptr %str_slot502)
  %match_str_bool504 = icmp ne i32 %match_str_eq503, 0
  br i1 %match_str_bool504, label %match_arm_72, label %match_test_73

match_arm_72:                                     ; preds = %match_test_72
  %Token_ctor505 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor505, align 1
  %tag_ptr506 = getelementptr inbounds %Token, ptr %Token_ctor505, i32 0, i32 0
  store i8 52, ptr %tag_ptr506, align 1
  %enum_val507 = load %Token, ptr %Token_ctor505, align 1
  ret %Token %enum_val507

match_test_73:                                    ; preds = %match_test_72
  %str_slot508 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot508, align 8
  %str_slot509 = alloca %NomString, align 8
  store %NomString { ptr @str.78, i64 5 }, ptr %str_slot509, align 8
  %match_str_eq510 = call i32 @nom_string_eq(ptr %str_slot508, ptr %str_slot509)
  %match_str_bool511 = icmp ne i32 %match_str_eq510, 0
  br i1 %match_str_bool511, label %match_arm_73, label %match_test_74

match_arm_73:                                     ; preds = %match_test_73
  %Token_ctor512 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor512, align 1
  %tag_ptr513 = getelementptr inbounds %Token, ptr %Token_ctor512, i32 0, i32 0
  store i8 55, ptr %tag_ptr513, align 1
  %enum_val514 = load %Token, ptr %Token_ctor512, align 1
  ret %Token %enum_val514

match_test_74:                                    ; preds = %match_test_73
  %str_slot515 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot515, align 8
  %str_slot516 = alloca %NomString, align 8
  store %NomString { ptr @str.79, i64 8 }, ptr %str_slot516, align 8
  %match_str_eq517 = call i32 @nom_string_eq(ptr %str_slot515, ptr %str_slot516)
  %match_str_bool518 = icmp ne i32 %match_str_eq517, 0
  br i1 %match_str_bool518, label %match_arm_74, label %match_test_75

match_arm_74:                                     ; preds = %match_test_74
  %Token_ctor519 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor519, align 1
  %tag_ptr520 = getelementptr inbounds %Token, ptr %Token_ctor519, i32 0, i32 0
  store i8 56, ptr %tag_ptr520, align 1
  %enum_val521 = load %Token, ptr %Token_ctor519, align 1
  ret %Token %enum_val521

match_test_75:                                    ; preds = %match_test_74
  %str_slot522 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot522, align 8
  %str_slot523 = alloca %NomString, align 8
  store %NomString { ptr @str.80, i64 5 }, ptr %str_slot523, align 8
  %match_str_eq524 = call i32 @nom_string_eq(ptr %str_slot522, ptr %str_slot523)
  %match_str_bool525 = icmp ne i32 %match_str_eq524, 0
  br i1 %match_str_bool525, label %match_arm_75, label %match_test_76

match_arm_75:                                     ; preds = %match_test_75
  %Token_ctor526 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor526, align 1
  %tag_ptr527 = getelementptr inbounds %Token, ptr %Token_ctor526, i32 0, i32 0
  store i8 57, ptr %tag_ptr527, align 1
  %enum_val528 = load %Token, ptr %Token_ctor526, align 1
  ret %Token %enum_val528

match_test_76:                                    ; preds = %match_test_75
  %str_slot529 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot529, align 8
  %str_slot530 = alloca %NomString, align 8
  store %NomString { ptr @str.81, i64 4 }, ptr %str_slot530, align 8
  %match_str_eq531 = call i32 @nom_string_eq(ptr %str_slot529, ptr %str_slot530)
  %match_str_bool532 = icmp ne i32 %match_str_eq531, 0
  br i1 %match_str_bool532, label %match_arm_76, label %match_test_77

match_arm_76:                                     ; preds = %match_test_76
  %Token_ctor533 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor533, align 1
  %tag_ptr534 = getelementptr inbounds %Token, ptr %Token_ctor533, i32 0, i32 0
  store i8 93, ptr %tag_ptr534, align 1
  %payload_ptr = getelementptr inbounds %Token, ptr %Token_ctor533, i32 0, i32 1
  store i1 true, ptr %payload_ptr, align 1
  %enum_val535 = load %Token, ptr %Token_ctor533, align 1
  ret %Token %enum_val535

match_test_77:                                    ; preds = %match_test_76
  %str_slot536 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot536, align 8
  %str_slot537 = alloca %NomString, align 8
  store %NomString { ptr @str.82, i64 5 }, ptr %str_slot537, align 8
  %match_str_eq538 = call i32 @nom_string_eq(ptr %str_slot536, ptr %str_slot537)
  %match_str_bool539 = icmp ne i32 %match_str_eq538, 0
  br i1 %match_str_bool539, label %match_arm_77, label %match_test_78

match_arm_77:                                     ; preds = %match_test_77
  %Token_ctor540 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor540, align 1
  %tag_ptr541 = getelementptr inbounds %Token, ptr %Token_ctor540, i32 0, i32 0
  store i8 93, ptr %tag_ptr541, align 1
  %payload_ptr542 = getelementptr inbounds %Token, ptr %Token_ctor540, i32 0, i32 1
  store i1 false, ptr %payload_ptr542, align 1
  %enum_val543 = load %Token, ptr %Token_ctor540, align 1
  ret %Token %enum_val543

match_test_78:                                    ; preds = %match_test_77
  %str_slot544 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot544, align 8
  %str_slot545 = alloca %NomString, align 8
  store %NomString { ptr @str.83, i64 3 }, ptr %str_slot545, align 8
  %match_str_eq546 = call i32 @nom_string_eq(ptr %str_slot544, ptr %str_slot545)
  %match_str_bool547 = icmp ne i32 %match_str_eq546, 0
  br i1 %match_str_bool547, label %match_arm_78, label %match_test_79

match_arm_78:                                     ; preds = %match_test_78
  %Token_ctor548 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor548, align 1
  %tag_ptr549 = getelementptr inbounds %Token, ptr %Token_ctor548, i32 0, i32 0
  store i8 93, ptr %tag_ptr549, align 1
  %payload_ptr550 = getelementptr inbounds %Token, ptr %Token_ctor548, i32 0, i32 1
  store i1 true, ptr %payload_ptr550, align 1
  %enum_val551 = load %Token, ptr %Token_ctor548, align 1
  ret %Token %enum_val551

match_test_79:                                    ; preds = %match_test_78
  %str_slot552 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot552, align 8
  %str_slot553 = alloca %NomString, align 8
  store %NomString { ptr @str.84, i64 2 }, ptr %str_slot553, align 8
  %match_str_eq554 = call i32 @nom_string_eq(ptr %str_slot552, ptr %str_slot553)
  %match_str_bool555 = icmp ne i32 %match_str_eq554, 0
  br i1 %match_str_bool555, label %match_arm_79, label %match_test_80

match_arm_79:                                     ; preds = %match_test_79
  %Token_ctor556 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor556, align 1
  %tag_ptr557 = getelementptr inbounds %Token, ptr %Token_ctor556, i32 0, i32 0
  store i8 93, ptr %tag_ptr557, align 1
  %payload_ptr558 = getelementptr inbounds %Token, ptr %Token_ctor556, i32 0, i32 1
  store i1 false, ptr %payload_ptr558, align 1
  %enum_val559 = load %Token, ptr %Token_ctor556, align 1
  ret %Token %enum_val559

match_test_80:                                    ; preds = %match_test_79
  %str_slot560 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot560, align 8
  %str_slot561 = alloca %NomString, align 8
  store %NomString { ptr @str.85, i64 4 }, ptr %str_slot561, align 8
  %match_str_eq562 = call i32 @nom_string_eq(ptr %str_slot560, ptr %str_slot561)
  %match_str_bool563 = icmp ne i32 %match_str_eq562, 0
  br i1 %match_str_bool563, label %match_arm_80, label %match_test_81

match_arm_80:                                     ; preds = %match_test_80
  %Token_ctor564 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor564, align 1
  %tag_ptr565 = getelementptr inbounds %Token, ptr %Token_ctor564, i32 0, i32 0
  store i8 94, ptr %tag_ptr565, align 1
  %enum_val566 = load %Token, ptr %Token_ctor564, align 1
  ret %Token %enum_val566

match_test_81:                                    ; preds = %match_test_80
  %str_slot567 = alloca %NomString, align 8
  store %NomString %word2, ptr %str_slot567, align 8
  %str_slot568 = alloca %NomString, align 8
  store %NomString { ptr @str.86, i64 7 }, ptr %str_slot568, align 8
  %match_str_eq569 = call i32 @nom_string_eq(ptr %str_slot567, ptr %str_slot568)
  %match_str_bool570 = icmp ne i32 %match_str_eq569, 0
  br i1 %match_str_bool570, label %match_arm_81, label %match_test_82

match_arm_81:                                     ; preds = %match_test_81
  %Token_ctor571 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor571, align 1
  %tag_ptr572 = getelementptr inbounds %Token, ptr %Token_ctor571, i32 0, i32 0
  store i8 94, ptr %tag_ptr572, align 1
  %enum_val573 = load %Token, ptr %Token_ctor571, align 1
  ret %Token %enum_val573

match_test_82:                                    ; preds = %match_test_81
  br label %match_arm_82

match_arm_82:                                     ; preds = %match_test_82
  %word574 = load %NomString, ptr %word, align 8
  %Token_ctor575 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor575, align 1
  %tag_ptr576 = getelementptr inbounds %Token, ptr %Token_ctor575, i32 0, i32 0
  store i8 95, ptr %tag_ptr576, align 1
  %payload_ptr577 = getelementptr inbounds %Token, ptr %Token_ctor575, i32 0, i32 1
  store %NomString %word574, ptr %payload_ptr577, align 8
  %enum_val578 = load %Token, ptr %Token_ctor575, align 1
  ret %Token %enum_val578
}

define { %SpannedToken, %Lexer } @next_token(%Lexer %lex1) {
entry:
  %lex = alloca %Lexer, align 8
  store %Lexer %lex1, ptr %lex, align 8
  %lex2 = load %Lexer, ptr %lex, align 8
  %call = call %Lexer @skip_horizontal_ws(%Lexer %lex2)
  %l = alloca %Lexer, align 8
  store %Lexer %call, ptr %l, align 8
  %l3 = load %Lexer, ptr %l, align 8
  %call4 = call i1 @is_at_end(%Lexer %l3)
  br i1 %call4, label %then, label %else

then:                                             ; preds = %entry
  %Span_init = alloca %Span, align 8
  store %Span zeroinitializer, ptr %Span_init, align 4
  %l.pos.ptr = getelementptr inbounds %Lexer, ptr %l, i32 0, i32 1
  %l.pos = load i64, ptr %l.pos.ptr, align 4
  %Span.start.init = getelementptr inbounds %Span, ptr %Span_init, i32 0, i32 0
  store i64 %l.pos, ptr %Span.start.init, align 4
  %l.pos.ptr5 = getelementptr inbounds %Lexer, ptr %l, i32 0, i32 1
  %l.pos6 = load i64, ptr %l.pos.ptr5, align 4
  %Span.end.init = getelementptr inbounds %Span, ptr %Span_init, i32 0, i32 1
  store i64 %l.pos6, ptr %Span.end.init, align 4
  %l.line.ptr = getelementptr inbounds %Lexer, ptr %l, i32 0, i32 2
  %l.line = load i64, ptr %l.line.ptr, align 4
  %Span.line.init = getelementptr inbounds %Span, ptr %Span_init, i32 0, i32 2
  store i64 %l.line, ptr %Span.line.init, align 4
  %l.col.ptr = getelementptr inbounds %Lexer, ptr %l, i32 0, i32 3
  %l.col = load i64, ptr %l.col.ptr, align 4
  %Span.col.init = getelementptr inbounds %Span, ptr %Span_init, i32 0, i32 3
  store i64 %l.col, ptr %Span.col.init, align 4
  %Span_val = load %Span, ptr %Span_init, align 4
  %sp = alloca %Span, align 8
  store %Span %Span_val, ptr %sp, align 4
  %SpannedToken_init = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init, align 4
  %Token_ctor = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor, align 1
  %tag_ptr = getelementptr inbounds %Token, ptr %Token_ctor, i32 0, i32 0
  store i8 99, ptr %tag_ptr, align 1
  %enum_val = load %Token, ptr %Token_ctor, align 1
  %SpannedToken.token.init = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init, i32 0, i32 0
  store %Token %enum_val, ptr %SpannedToken.token.init, align 1
  %sp7 = load %Span, ptr %sp, align 4
  %SpannedToken.span.init = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init, i32 0, i32 1
  store %Span %sp7, ptr %SpannedToken.span.init, align 4
  %SpannedToken_val = load %SpannedToken, ptr %SpannedToken_init, align 4
  %l8 = load %Lexer, ptr %l, align 8
  %tup0 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val, 0
  %tup1 = insertvalue { %SpannedToken, %Lexer } %tup0, %Lexer %l8, 1
  ret { %SpannedToken, %Lexer } %tup1

else:                                             ; preds = %entry
  br label %ifcont

ifcont:                                           ; preds = %else
  %l.pos.ptr9 = getelementptr inbounds %Lexer, ptr %l, i32 0, i32 1
  %l.pos10 = load i64, ptr %l.pos.ptr9, align 4
  %start_pos = alloca i64, align 8
  store i64 %l.pos10, ptr %start_pos, align 4
  %l.line.ptr11 = getelementptr inbounds %Lexer, ptr %l, i32 0, i32 2
  %l.line12 = load i64, ptr %l.line.ptr11, align 4
  %start_line = alloca i64, align 8
  store i64 %l.line12, ptr %start_line, align 4
  %l.col.ptr13 = getelementptr inbounds %Lexer, ptr %l, i32 0, i32 3
  %l.col14 = load i64, ptr %l.col.ptr13, align 4
  %start_col = alloca i64, align 8
  store i64 %l.col14, ptr %start_col, align 4
  %l15 = load %Lexer, ptr %l, align 8
  %call16 = call i64 @current_char(%Lexer %l15)
  %ch = alloca i64, align 8
  store i64 %call16, ptr %ch, align 4
  %ch17 = load i64, ptr %ch, align 4
  %icmp = icmp eq i64 %ch17, 10
  br i1 %icmp, label %then18, label %else19

then18:                                           ; preds = %ifcont
  %l21 = load %Lexer, ptr %l, align 8
  %call22 = call %Lexer @advance(%Lexer %l21)
  %l2 = alloca %Lexer, align 8
  store %Lexer %call22, ptr %l2, align 8
  %start_pos23 = load i64, ptr %start_pos, align 4
  %start_line24 = load i64, ptr %start_line, align 4
  %start_col25 = load i64, ptr %start_col, align 4
  %l226 = load %Lexer, ptr %l2, align 8
  %call27 = call %Span @make_span(i64 %start_pos23, i64 %start_line24, i64 %start_col25, %Lexer %l226)
  %sp28 = alloca %Span, align 8
  store %Span %call27, ptr %sp28, align 4
  %SpannedToken_init29 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init29, align 4
  %Token_ctor30 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor30, align 1
  %tag_ptr31 = getelementptr inbounds %Token, ptr %Token_ctor30, i32 0, i32 0
  store i8 96, ptr %tag_ptr31, align 1
  %enum_val32 = load %Token, ptr %Token_ctor30, align 1
  %SpannedToken.token.init33 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init29, i32 0, i32 0
  store %Token %enum_val32, ptr %SpannedToken.token.init33, align 1
  %sp34 = load %Span, ptr %sp28, align 4
  %SpannedToken.span.init35 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init29, i32 0, i32 1
  store %Span %sp34, ptr %SpannedToken.span.init35, align 4
  %SpannedToken_val36 = load %SpannedToken, ptr %SpannedToken_init29, align 4
  %l237 = load %Lexer, ptr %l2, align 8
  %tup038 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val36, 0
  %tup139 = insertvalue { %SpannedToken, %Lexer } %tup038, %Lexer %l237, 1
  ret { %SpannedToken, %Lexer } %tup139

else19:                                           ; preds = %ifcont
  br label %ifcont20

ifcont20:                                         ; preds = %else19
  %ch40 = load i64, ptr %ch, align 4
  %icmp41 = icmp eq i64 %ch40, 35
  br i1 %icmp41, label %then42, label %else43

then42:                                           ; preds = %ifcont20
  %l45 = load %Lexer, ptr %l, align 8
  %call46 = call %Lexer @advance(%Lexer %l45)
  %l247 = alloca %Lexer, align 8
  store %Lexer %call46, ptr %l247, align 8
  %l248 = load %Lexer, ptr %l247, align 8
  %call49 = call { %NomString, %Lexer } @read_comment(%Lexer %l248)
  %pair1 = alloca { %NomString, %Lexer }, align 8
  store { %NomString, %Lexer } %call49, ptr %pair1, align 8
  %pair150 = load { %NomString, %Lexer }, ptr %pair1, align 8
  %pair1.0 = extractvalue { %NomString, %Lexer } %pair150, 0
  %content = alloca %NomString, align 8
  store %NomString %pair1.0, ptr %content, align 8
  %pair151 = load { %NomString, %Lexer }, ptr %pair1, align 8
  %pair1.1 = extractvalue { %NomString, %Lexer } %pair151, 1
  %l352 = alloca %Lexer, align 8
  store %Lexer %pair1.1, ptr %l352, align 8
  %start_pos53 = load i64, ptr %start_pos, align 4
  %start_line54 = load i64, ptr %start_line, align 4
  %start_col55 = load i64, ptr %start_col, align 4
  %l356 = load %Lexer, ptr %l352, align 8
  %call57 = call %Span @make_span(i64 %start_pos53, i64 %start_line54, i64 %start_col55, %Lexer %l356)
  %sp58 = alloca %Span, align 8
  store %Span %call57, ptr %sp58, align 4
  %SpannedToken_init59 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init59, align 4
  %content60 = load %NomString, ptr %content, align 8
  %Token_ctor61 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor61, align 1
  %tag_ptr62 = getelementptr inbounds %Token, ptr %Token_ctor61, i32 0, i32 0
  store i8 98, ptr %tag_ptr62, align 1
  %payload_ptr = getelementptr inbounds %Token, ptr %Token_ctor61, i32 0, i32 1
  store %NomString %content60, ptr %payload_ptr, align 8
  %enum_val63 = load %Token, ptr %Token_ctor61, align 1
  %SpannedToken.token.init64 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init59, i32 0, i32 0
  store %Token %enum_val63, ptr %SpannedToken.token.init64, align 1
  %sp65 = load %Span, ptr %sp58, align 4
  %SpannedToken.span.init66 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init59, i32 0, i32 1
  store %Span %sp65, ptr %SpannedToken.span.init66, align 4
  %SpannedToken_val67 = load %SpannedToken, ptr %SpannedToken_init59, align 4
  %l368 = load %Lexer, ptr %l352, align 8
  %tup069 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val67, 0
  %tup170 = insertvalue { %SpannedToken, %Lexer } %tup069, %Lexer %l368, 1
  ret { %SpannedToken, %Lexer } %tup170

else43:                                           ; preds = %ifcont20
  br label %ifcont44

ifcont44:                                         ; preds = %else43
  %ch71 = load i64, ptr %ch, align 4
  %icmp72 = icmp eq i64 %ch71, 34
  br i1 %icmp72, label %then73, label %else74

then73:                                           ; preds = %ifcont44
  %l76 = load %Lexer, ptr %l, align 8
  %call77 = call %Lexer @advance(%Lexer %l76)
  %l278 = alloca %Lexer, align 8
  store %Lexer %call77, ptr %l278, align 8
  %l279 = load %Lexer, ptr %l278, align 8
  %call80 = call { %NomString, %Lexer } @read_string(%Lexer %l279)
  %pair2 = alloca { %NomString, %Lexer }, align 8
  store { %NomString, %Lexer } %call80, ptr %pair2, align 8
  %pair281 = load { %NomString, %Lexer }, ptr %pair2, align 8
  %pair2.0 = extractvalue { %NomString, %Lexer } %pair281, 0
  %content82 = alloca %NomString, align 8
  store %NomString %pair2.0, ptr %content82, align 8
  %pair283 = load { %NomString, %Lexer }, ptr %pair2, align 8
  %pair2.1 = extractvalue { %NomString, %Lexer } %pair283, 1
  %l384 = alloca %Lexer, align 8
  store %Lexer %pair2.1, ptr %l384, align 8
  %start_pos85 = load i64, ptr %start_pos, align 4
  %start_line86 = load i64, ptr %start_line, align 4
  %start_col87 = load i64, ptr %start_col, align 4
  %l388 = load %Lexer, ptr %l384, align 8
  %call89 = call %Span @make_span(i64 %start_pos85, i64 %start_line86, i64 %start_col87, %Lexer %l388)
  %sp90 = alloca %Span, align 8
  store %Span %call89, ptr %sp90, align 4
  %SpannedToken_init91 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init91, align 4
  %content92 = load %NomString, ptr %content82, align 8
  %Token_ctor93 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor93, align 1
  %tag_ptr94 = getelementptr inbounds %Token, ptr %Token_ctor93, i32 0, i32 0
  store i8 92, ptr %tag_ptr94, align 1
  %payload_ptr95 = getelementptr inbounds %Token, ptr %Token_ctor93, i32 0, i32 1
  store %NomString %content92, ptr %payload_ptr95, align 8
  %enum_val96 = load %Token, ptr %Token_ctor93, align 1
  %SpannedToken.token.init97 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init91, i32 0, i32 0
  store %Token %enum_val96, ptr %SpannedToken.token.init97, align 1
  %sp98 = load %Span, ptr %sp90, align 4
  %SpannedToken.span.init99 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init91, i32 0, i32 1
  store %Span %sp98, ptr %SpannedToken.span.init99, align 4
  %SpannedToken_val100 = load %SpannedToken, ptr %SpannedToken_init91, align 4
  %l3101 = load %Lexer, ptr %l384, align 8
  %tup0102 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val100, 0
  %tup1103 = insertvalue { %SpannedToken, %Lexer } %tup0102, %Lexer %l3101, 1
  ret { %SpannedToken, %Lexer } %tup1103

else74:                                           ; preds = %ifcont44
  br label %ifcont75

ifcont75:                                         ; preds = %else74
  %ch104 = load i64, ptr %ch, align 4
  %call105 = call i1 @is_digit(i64 %ch104)
  br i1 %call105, label %then106, label %else107

then106:                                          ; preds = %ifcont75
  %l109 = load %Lexer, ptr %l, align 8
  %call110 = call { %Token, %Lexer } @read_number(%Lexer %l109)
  %pair3 = alloca { %Token, %Lexer }, align 8
  store { %Token, %Lexer } %call110, ptr %pair3, align 8
  %pair3111 = load { %Token, %Lexer }, ptr %pair3, align 8
  %pair3.0 = extractvalue { %Token, %Lexer } %pair3111, 0
  %tok = alloca %Token, align 8
  store %Token %pair3.0, ptr %tok, align 1
  %pair3112 = load { %Token, %Lexer }, ptr %pair3, align 8
  %pair3.1 = extractvalue { %Token, %Lexer } %pair3112, 1
  %l2113 = alloca %Lexer, align 8
  store %Lexer %pair3.1, ptr %l2113, align 8
  %start_pos114 = load i64, ptr %start_pos, align 4
  %start_line115 = load i64, ptr %start_line, align 4
  %start_col116 = load i64, ptr %start_col, align 4
  %l2117 = load %Lexer, ptr %l2113, align 8
  %call118 = call %Span @make_span(i64 %start_pos114, i64 %start_line115, i64 %start_col116, %Lexer %l2117)
  %sp119 = alloca %Span, align 8
  store %Span %call118, ptr %sp119, align 4
  %SpannedToken_init120 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init120, align 4
  %tok121 = load %Token, ptr %tok, align 1
  %SpannedToken.token.init122 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init120, i32 0, i32 0
  store %Token %tok121, ptr %SpannedToken.token.init122, align 1
  %sp123 = load %Span, ptr %sp119, align 4
  %SpannedToken.span.init124 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init120, i32 0, i32 1
  store %Span %sp123, ptr %SpannedToken.span.init124, align 4
  %SpannedToken_val125 = load %SpannedToken, ptr %SpannedToken_init120, align 4
  %l2126 = load %Lexer, ptr %l2113, align 8
  %tup0127 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val125, 0
  %tup1128 = insertvalue { %SpannedToken, %Lexer } %tup0127, %Lexer %l2126, 1
  ret { %SpannedToken, %Lexer } %tup1128

else107:                                          ; preds = %ifcont75
  br label %ifcont108

ifcont108:                                        ; preds = %else107
  %ch129 = load i64, ptr %ch, align 4
  %call130 = call i1 @is_alpha(i64 %ch129)
  br i1 %call130, label %then131, label %else132

then131:                                          ; preds = %ifcont108
  %l134 = load %Lexer, ptr %l, align 8
  %call135 = call { %NomString, %Lexer } @read_word(%Lexer %l134)
  %pair4 = alloca { %NomString, %Lexer }, align 8
  store { %NomString, %Lexer } %call135, ptr %pair4, align 8
  %pair4136 = load { %NomString, %Lexer }, ptr %pair4, align 8
  %pair4.0 = extractvalue { %NomString, %Lexer } %pair4136, 0
  %word = alloca %NomString, align 8
  store %NomString %pair4.0, ptr %word, align 8
  %pair4137 = load { %NomString, %Lexer }, ptr %pair4, align 8
  %pair4.1 = extractvalue { %NomString, %Lexer } %pair4137, 1
  %l2138 = alloca %Lexer, align 8
  store %Lexer %pair4.1, ptr %l2138, align 8
  %word139 = load %NomString, ptr %word, align 8
  %call140 = call %Token @classify_word(%NomString %word139)
  %tok141 = alloca %Token, align 8
  store %Token %call140, ptr %tok141, align 1
  %start_pos142 = load i64, ptr %start_pos, align 4
  %start_line143 = load i64, ptr %start_line, align 4
  %start_col144 = load i64, ptr %start_col, align 4
  %l2145 = load %Lexer, ptr %l2138, align 8
  %call146 = call %Span @make_span(i64 %start_pos142, i64 %start_line143, i64 %start_col144, %Lexer %l2145)
  %sp147 = alloca %Span, align 8
  store %Span %call146, ptr %sp147, align 4
  %SpannedToken_init148 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init148, align 4
  %tok149 = load %Token, ptr %tok141, align 1
  %SpannedToken.token.init150 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init148, i32 0, i32 0
  store %Token %tok149, ptr %SpannedToken.token.init150, align 1
  %sp151 = load %Span, ptr %sp147, align 4
  %SpannedToken.span.init152 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init148, i32 0, i32 1
  store %Span %sp151, ptr %SpannedToken.span.init152, align 4
  %SpannedToken_val153 = load %SpannedToken, ptr %SpannedToken_init148, align 4
  %l2154 = load %Lexer, ptr %l2138, align 8
  %tup0155 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val153, 0
  %tup1156 = insertvalue { %SpannedToken, %Lexer } %tup0155, %Lexer %l2154, 1
  ret { %SpannedToken, %Lexer } %tup1156

else132:                                          ; preds = %ifcont108
  br label %ifcont133

ifcont133:                                        ; preds = %else132
  %l157 = load %Lexer, ptr %l, align 8
  %call158 = call i64 @peek_next(%Lexer %l157)
  %next = alloca i64, align 8
  store i64 %call158, ptr %next, align 4
  %ch159 = load i64, ptr %ch, align 4
  %icmp160 = icmp eq i64 %ch159, 45
  %next161 = load i64, ptr %next, align 4
  %icmp162 = icmp eq i64 %next161, 62
  %and = and i1 %icmp160, %icmp162
  br i1 %and, label %then163, label %else164

then163:                                          ; preds = %ifcont133
  %l166 = load %Lexer, ptr %l, align 8
  %call167 = call %Lexer @advance(%Lexer %l166)
  %call168 = call %Lexer @advance(%Lexer %call167)
  %l2169 = alloca %Lexer, align 8
  store %Lexer %call168, ptr %l2169, align 8
  %start_pos170 = load i64, ptr %start_pos, align 4
  %start_line171 = load i64, ptr %start_line, align 4
  %start_col172 = load i64, ptr %start_col, align 4
  %l2173 = load %Lexer, ptr %l2169, align 8
  %call174 = call %Span @make_span(i64 %start_pos170, i64 %start_line171, i64 %start_col172, %Lexer %l2173)
  %sp175 = alloca %Span, align 8
  store %Span %call174, ptr %sp175, align 4
  %SpannedToken_init176 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init176, align 4
  %Token_ctor177 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor177, align 1
  %tag_ptr178 = getelementptr inbounds %Token, ptr %Token_ctor177, i32 0, i32 0
  store i8 61, ptr %tag_ptr178, align 1
  %enum_val179 = load %Token, ptr %Token_ctor177, align 1
  %SpannedToken.token.init180 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init176, i32 0, i32 0
  store %Token %enum_val179, ptr %SpannedToken.token.init180, align 1
  %sp181 = load %Span, ptr %sp175, align 4
  %SpannedToken.span.init182 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init176, i32 0, i32 1
  store %Span %sp181, ptr %SpannedToken.span.init182, align 4
  %SpannedToken_val183 = load %SpannedToken, ptr %SpannedToken_init176, align 4
  %l2184 = load %Lexer, ptr %l2169, align 8
  %tup0185 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val183, 0
  %tup1186 = insertvalue { %SpannedToken, %Lexer } %tup0185, %Lexer %l2184, 1
  ret { %SpannedToken, %Lexer } %tup1186

else164:                                          ; preds = %ifcont133
  br label %ifcont165

ifcont165:                                        ; preds = %else164
  %ch187 = load i64, ptr %ch, align 4
  %icmp188 = icmp eq i64 %ch187, 61
  %next189 = load i64, ptr %next, align 4
  %icmp190 = icmp eq i64 %next189, 62
  %and191 = and i1 %icmp188, %icmp190
  br i1 %and191, label %then192, label %else193

then192:                                          ; preds = %ifcont165
  %l195 = load %Lexer, ptr %l, align 8
  %call196 = call %Lexer @advance(%Lexer %l195)
  %call197 = call %Lexer @advance(%Lexer %call196)
  %l2198 = alloca %Lexer, align 8
  store %Lexer %call197, ptr %l2198, align 8
  %start_pos199 = load i64, ptr %start_pos, align 4
  %start_line200 = load i64, ptr %start_line, align 4
  %start_col201 = load i64, ptr %start_col, align 4
  %l2202 = load %Lexer, ptr %l2198, align 8
  %call203 = call %Span @make_span(i64 %start_pos199, i64 %start_line200, i64 %start_col201, %Lexer %l2202)
  %sp204 = alloca %Span, align 8
  store %Span %call203, ptr %sp204, align 4
  %SpannedToken_init205 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init205, align 4
  %Token_ctor206 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor206, align 1
  %tag_ptr207 = getelementptr inbounds %Token, ptr %Token_ctor206, i32 0, i32 0
  store i8 62, ptr %tag_ptr207, align 1
  %enum_val208 = load %Token, ptr %Token_ctor206, align 1
  %SpannedToken.token.init209 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init205, i32 0, i32 0
  store %Token %enum_val208, ptr %SpannedToken.token.init209, align 1
  %sp210 = load %Span, ptr %sp204, align 4
  %SpannedToken.span.init211 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init205, i32 0, i32 1
  store %Span %sp210, ptr %SpannedToken.span.init211, align 4
  %SpannedToken_val212 = load %SpannedToken, ptr %SpannedToken_init205, align 4
  %l2213 = load %Lexer, ptr %l2198, align 8
  %tup0214 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val212, 0
  %tup1215 = insertvalue { %SpannedToken, %Lexer } %tup0214, %Lexer %l2213, 1
  ret { %SpannedToken, %Lexer } %tup1215

else193:                                          ; preds = %ifcont165
  br label %ifcont194

ifcont194:                                        ; preds = %else193
  %ch216 = load i64, ptr %ch, align 4
  %icmp217 = icmp eq i64 %ch216, 58
  %next218 = load i64, ptr %next, align 4
  %icmp219 = icmp eq i64 %next218, 58
  %and220 = and i1 %icmp217, %icmp219
  br i1 %and220, label %then221, label %else222

then221:                                          ; preds = %ifcont194
  %l224 = load %Lexer, ptr %l, align 8
  %call225 = call %Lexer @advance(%Lexer %l224)
  %call226 = call %Lexer @advance(%Lexer %call225)
  %l2227 = alloca %Lexer, align 8
  store %Lexer %call226, ptr %l2227, align 8
  %start_pos228 = load i64, ptr %start_pos, align 4
  %start_line229 = load i64, ptr %start_line, align 4
  %start_col230 = load i64, ptr %start_col, align 4
  %l2231 = load %Lexer, ptr %l2227, align 8
  %call232 = call %Span @make_span(i64 %start_pos228, i64 %start_line229, i64 %start_col230, %Lexer %l2231)
  %sp233 = alloca %Span, align 8
  store %Span %call232, ptr %sp233, align 4
  %SpannedToken_init234 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init234, align 4
  %Token_ctor235 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor235, align 1
  %tag_ptr236 = getelementptr inbounds %Token, ptr %Token_ctor235, i32 0, i32 0
  store i8 63, ptr %tag_ptr236, align 1
  %enum_val237 = load %Token, ptr %Token_ctor235, align 1
  %SpannedToken.token.init238 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init234, i32 0, i32 0
  store %Token %enum_val237, ptr %SpannedToken.token.init238, align 1
  %sp239 = load %Span, ptr %sp233, align 4
  %SpannedToken.span.init240 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init234, i32 0, i32 1
  store %Span %sp239, ptr %SpannedToken.span.init240, align 4
  %SpannedToken_val241 = load %SpannedToken, ptr %SpannedToken_init234, align 4
  %l2242 = load %Lexer, ptr %l2227, align 8
  %tup0243 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val241, 0
  %tup1244 = insertvalue { %SpannedToken, %Lexer } %tup0243, %Lexer %l2242, 1
  ret { %SpannedToken, %Lexer } %tup1244

else222:                                          ; preds = %ifcont194
  br label %ifcont223

ifcont223:                                        ; preds = %else222
  %ch245 = load i64, ptr %ch, align 4
  %icmp246 = icmp eq i64 %ch245, 62
  %next247 = load i64, ptr %next, align 4
  %icmp248 = icmp eq i64 %next247, 61
  %and249 = and i1 %icmp246, %icmp248
  br i1 %and249, label %then250, label %else251

then250:                                          ; preds = %ifcont223
  %l253 = load %Lexer, ptr %l, align 8
  %call254 = call %Lexer @advance(%Lexer %l253)
  %call255 = call %Lexer @advance(%Lexer %call254)
  %l2256 = alloca %Lexer, align 8
  store %Lexer %call255, ptr %l2256, align 8
  %start_pos257 = load i64, ptr %start_pos, align 4
  %start_line258 = load i64, ptr %start_line, align 4
  %start_col259 = load i64, ptr %start_col, align 4
  %l2260 = load %Lexer, ptr %l2256, align 8
  %call261 = call %Span @make_span(i64 %start_pos257, i64 %start_line258, i64 %start_col259, %Lexer %l2260)
  %sp262 = alloca %Span, align 8
  store %Span %call261, ptr %sp262, align 4
  %SpannedToken_init263 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init263, align 4
  %Token_ctor264 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor264, align 1
  %tag_ptr265 = getelementptr inbounds %Token, ptr %Token_ctor264, i32 0, i32 0
  store i8 71, ptr %tag_ptr265, align 1
  %enum_val266 = load %Token, ptr %Token_ctor264, align 1
  %SpannedToken.token.init267 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init263, i32 0, i32 0
  store %Token %enum_val266, ptr %SpannedToken.token.init267, align 1
  %sp268 = load %Span, ptr %sp262, align 4
  %SpannedToken.span.init269 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init263, i32 0, i32 1
  store %Span %sp268, ptr %SpannedToken.span.init269, align 4
  %SpannedToken_val270 = load %SpannedToken, ptr %SpannedToken_init263, align 4
  %l2271 = load %Lexer, ptr %l2256, align 8
  %tup0272 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val270, 0
  %tup1273 = insertvalue { %SpannedToken, %Lexer } %tup0272, %Lexer %l2271, 1
  ret { %SpannedToken, %Lexer } %tup1273

else251:                                          ; preds = %ifcont223
  br label %ifcont252

ifcont252:                                        ; preds = %else251
  %ch274 = load i64, ptr %ch, align 4
  %icmp275 = icmp eq i64 %ch274, 60
  %next276 = load i64, ptr %next, align 4
  %icmp277 = icmp eq i64 %next276, 61
  %and278 = and i1 %icmp275, %icmp277
  br i1 %and278, label %then279, label %else280

then279:                                          ; preds = %ifcont252
  %l282 = load %Lexer, ptr %l, align 8
  %call283 = call %Lexer @advance(%Lexer %l282)
  %call284 = call %Lexer @advance(%Lexer %call283)
  %l2285 = alloca %Lexer, align 8
  store %Lexer %call284, ptr %l2285, align 8
  %start_pos286 = load i64, ptr %start_pos, align 4
  %start_line287 = load i64, ptr %start_line, align 4
  %start_col288 = load i64, ptr %start_col, align 4
  %l2289 = load %Lexer, ptr %l2285, align 8
  %call290 = call %Span @make_span(i64 %start_pos286, i64 %start_line287, i64 %start_col288, %Lexer %l2289)
  %sp291 = alloca %Span, align 8
  store %Span %call290, ptr %sp291, align 4
  %SpannedToken_init292 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init292, align 4
  %Token_ctor293 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor293, align 1
  %tag_ptr294 = getelementptr inbounds %Token, ptr %Token_ctor293, i32 0, i32 0
  store i8 72, ptr %tag_ptr294, align 1
  %enum_val295 = load %Token, ptr %Token_ctor293, align 1
  %SpannedToken.token.init296 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init292, i32 0, i32 0
  store %Token %enum_val295, ptr %SpannedToken.token.init296, align 1
  %sp297 = load %Span, ptr %sp291, align 4
  %SpannedToken.span.init298 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init292, i32 0, i32 1
  store %Span %sp297, ptr %SpannedToken.span.init298, align 4
  %SpannedToken_val299 = load %SpannedToken, ptr %SpannedToken_init292, align 4
  %l2300 = load %Lexer, ptr %l2285, align 8
  %tup0301 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val299, 0
  %tup1302 = insertvalue { %SpannedToken, %Lexer } %tup0301, %Lexer %l2300, 1
  ret { %SpannedToken, %Lexer } %tup1302

else280:                                          ; preds = %ifcont252
  br label %ifcont281

ifcont281:                                        ; preds = %else280
  %ch303 = load i64, ptr %ch, align 4
  %icmp304 = icmp eq i64 %ch303, 61
  %next305 = load i64, ptr %next, align 4
  %icmp306 = icmp eq i64 %next305, 61
  %and307 = and i1 %icmp304, %icmp306
  br i1 %and307, label %then308, label %else309

then308:                                          ; preds = %ifcont281
  %l311 = load %Lexer, ptr %l, align 8
  %call312 = call %Lexer @advance(%Lexer %l311)
  %call313 = call %Lexer @advance(%Lexer %call312)
  %l2314 = alloca %Lexer, align 8
  store %Lexer %call313, ptr %l2314, align 8
  %start_pos315 = load i64, ptr %start_pos, align 4
  %start_line316 = load i64, ptr %start_line, align 4
  %start_col317 = load i64, ptr %start_col, align 4
  %l2318 = load %Lexer, ptr %l2314, align 8
  %call319 = call %Span @make_span(i64 %start_pos315, i64 %start_line316, i64 %start_col317, %Lexer %l2318)
  %sp320 = alloca %Span, align 8
  store %Span %call319, ptr %sp320, align 4
  %SpannedToken_init321 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init321, align 4
  %Token_ctor322 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor322, align 1
  %tag_ptr323 = getelementptr inbounds %Token, ptr %Token_ctor322, i32 0, i32 0
  store i8 74, ptr %tag_ptr323, align 1
  %enum_val324 = load %Token, ptr %Token_ctor322, align 1
  %SpannedToken.token.init325 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init321, i32 0, i32 0
  store %Token %enum_val324, ptr %SpannedToken.token.init325, align 1
  %sp326 = load %Span, ptr %sp320, align 4
  %SpannedToken.span.init327 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init321, i32 0, i32 1
  store %Span %sp326, ptr %SpannedToken.span.init327, align 4
  %SpannedToken_val328 = load %SpannedToken, ptr %SpannedToken_init321, align 4
  %l2329 = load %Lexer, ptr %l2314, align 8
  %tup0330 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val328, 0
  %tup1331 = insertvalue { %SpannedToken, %Lexer } %tup0330, %Lexer %l2329, 1
  ret { %SpannedToken, %Lexer } %tup1331

else309:                                          ; preds = %ifcont281
  br label %ifcont310

ifcont310:                                        ; preds = %else309
  %ch332 = load i64, ptr %ch, align 4
  %icmp333 = icmp eq i64 %ch332, 33
  %next334 = load i64, ptr %next, align 4
  %icmp335 = icmp eq i64 %next334, 61
  %and336 = and i1 %icmp333, %icmp335
  br i1 %and336, label %then337, label %else338

then337:                                          ; preds = %ifcont310
  %l340 = load %Lexer, ptr %l, align 8
  %call341 = call %Lexer @advance(%Lexer %l340)
  %call342 = call %Lexer @advance(%Lexer %call341)
  %l2343 = alloca %Lexer, align 8
  store %Lexer %call342, ptr %l2343, align 8
  %start_pos344 = load i64, ptr %start_pos, align 4
  %start_line345 = load i64, ptr %start_line, align 4
  %start_col346 = load i64, ptr %start_col, align 4
  %l2347 = load %Lexer, ptr %l2343, align 8
  %call348 = call %Span @make_span(i64 %start_pos344, i64 %start_line345, i64 %start_col346, %Lexer %l2347)
  %sp349 = alloca %Span, align 8
  store %Span %call348, ptr %sp349, align 4
  %SpannedToken_init350 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init350, align 4
  %Token_ctor351 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor351, align 1
  %tag_ptr352 = getelementptr inbounds %Token, ptr %Token_ctor351, i32 0, i32 0
  store i8 75, ptr %tag_ptr352, align 1
  %enum_val353 = load %Token, ptr %Token_ctor351, align 1
  %SpannedToken.token.init354 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init350, i32 0, i32 0
  store %Token %enum_val353, ptr %SpannedToken.token.init354, align 1
  %sp355 = load %Span, ptr %sp349, align 4
  %SpannedToken.span.init356 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init350, i32 0, i32 1
  store %Span %sp355, ptr %SpannedToken.span.init356, align 4
  %SpannedToken_val357 = load %SpannedToken, ptr %SpannedToken_init350, align 4
  %l2358 = load %Lexer, ptr %l2343, align 8
  %tup0359 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val357, 0
  %tup1360 = insertvalue { %SpannedToken, %Lexer } %tup0359, %Lexer %l2358, 1
  ret { %SpannedToken, %Lexer } %tup1360

else338:                                          ; preds = %ifcont310
  br label %ifcont339

ifcont339:                                        ; preds = %else338
  %l361 = load %Lexer, ptr %l, align 8
  %call362 = call %Lexer @advance(%Lexer %l361)
  %l2363 = alloca %Lexer, align 8
  store %Lexer %call362, ptr %l2363, align 8
  %start_pos364 = load i64, ptr %start_pos, align 4
  %start_line365 = load i64, ptr %start_line, align 4
  %start_col366 = load i64, ptr %start_col, align 4
  %l2367 = load %Lexer, ptr %l2363, align 8
  %call368 = call %Span @make_span(i64 %start_pos364, i64 %start_line365, i64 %start_col366, %Lexer %l2367)
  %sp369 = alloca %Span, align 8
  store %Span %call368, ptr %sp369, align 4
  %ch370 = load i64, ptr %ch, align 4
  br label %match_test_0

match_end:                                        ; No predecessors!
  ret { %SpannedToken, %Lexer } zeroinitializer

match_test_0:                                     ; preds = %ifcont339
  %match_cmp = icmp eq i64 %ch370, 43
  br i1 %match_cmp, label %match_arm_0, label %match_test_1

match_arm_0:                                      ; preds = %match_test_0
  %SpannedToken_init371 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init371, align 4
  %Token_ctor372 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor372, align 1
  %tag_ptr373 = getelementptr inbounds %Token, ptr %Token_ctor372, i32 0, i32 0
  store i8 64, ptr %tag_ptr373, align 1
  %enum_val374 = load %Token, ptr %Token_ctor372, align 1
  %SpannedToken.token.init375 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init371, i32 0, i32 0
  store %Token %enum_val374, ptr %SpannedToken.token.init375, align 1
  %sp376 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init377 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init371, i32 0, i32 1
  store %Span %sp376, ptr %SpannedToken.span.init377, align 4
  %SpannedToken_val378 = load %SpannedToken, ptr %SpannedToken_init371, align 4
  %l2379 = load %Lexer, ptr %l2363, align 8
  %tup0380 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val378, 0
  %tup1381 = insertvalue { %SpannedToken, %Lexer } %tup0380, %Lexer %l2379, 1
  ret { %SpannedToken, %Lexer } %tup1381

match_test_1:                                     ; preds = %match_test_0
  %match_cmp382 = icmp eq i64 %ch370, 45
  br i1 %match_cmp382, label %match_arm_1, label %match_test_2

match_arm_1:                                      ; preds = %match_test_1
  %SpannedToken_init383 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init383, align 4
  %Token_ctor384 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor384, align 1
  %tag_ptr385 = getelementptr inbounds %Token, ptr %Token_ctor384, i32 0, i32 0
  store i8 65, ptr %tag_ptr385, align 1
  %enum_val386 = load %Token, ptr %Token_ctor384, align 1
  %SpannedToken.token.init387 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init383, i32 0, i32 0
  store %Token %enum_val386, ptr %SpannedToken.token.init387, align 1
  %sp388 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init389 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init383, i32 0, i32 1
  store %Span %sp388, ptr %SpannedToken.span.init389, align 4
  %SpannedToken_val390 = load %SpannedToken, ptr %SpannedToken_init383, align 4
  %l2391 = load %Lexer, ptr %l2363, align 8
  %tup0392 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val390, 0
  %tup1393 = insertvalue { %SpannedToken, %Lexer } %tup0392, %Lexer %l2391, 1
  ret { %SpannedToken, %Lexer } %tup1393

match_test_2:                                     ; preds = %match_test_1
  %match_cmp394 = icmp eq i64 %ch370, 42
  br i1 %match_cmp394, label %match_arm_2, label %match_test_3

match_arm_2:                                      ; preds = %match_test_2
  %SpannedToken_init395 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init395, align 4
  %Token_ctor396 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor396, align 1
  %tag_ptr397 = getelementptr inbounds %Token, ptr %Token_ctor396, i32 0, i32 0
  store i8 66, ptr %tag_ptr397, align 1
  %enum_val398 = load %Token, ptr %Token_ctor396, align 1
  %SpannedToken.token.init399 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init395, i32 0, i32 0
  store %Token %enum_val398, ptr %SpannedToken.token.init399, align 1
  %sp400 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init401 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init395, i32 0, i32 1
  store %Span %sp400, ptr %SpannedToken.span.init401, align 4
  %SpannedToken_val402 = load %SpannedToken, ptr %SpannedToken_init395, align 4
  %l2403 = load %Lexer, ptr %l2363, align 8
  %tup0404 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val402, 0
  %tup1405 = insertvalue { %SpannedToken, %Lexer } %tup0404, %Lexer %l2403, 1
  ret { %SpannedToken, %Lexer } %tup1405

match_test_3:                                     ; preds = %match_test_2
  %match_cmp406 = icmp eq i64 %ch370, 47
  br i1 %match_cmp406, label %match_arm_3, label %match_test_4

match_arm_3:                                      ; preds = %match_test_3
  %SpannedToken_init407 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init407, align 4
  %Token_ctor408 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor408, align 1
  %tag_ptr409 = getelementptr inbounds %Token, ptr %Token_ctor408, i32 0, i32 0
  store i8 67, ptr %tag_ptr409, align 1
  %enum_val410 = load %Token, ptr %Token_ctor408, align 1
  %SpannedToken.token.init411 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init407, i32 0, i32 0
  store %Token %enum_val410, ptr %SpannedToken.token.init411, align 1
  %sp412 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init413 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init407, i32 0, i32 1
  store %Span %sp412, ptr %SpannedToken.span.init413, align 4
  %SpannedToken_val414 = load %SpannedToken, ptr %SpannedToken_init407, align 4
  %l2415 = load %Lexer, ptr %l2363, align 8
  %tup0416 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val414, 0
  %tup1417 = insertvalue { %SpannedToken, %Lexer } %tup0416, %Lexer %l2415, 1
  ret { %SpannedToken, %Lexer } %tup1417

match_test_4:                                     ; preds = %match_test_3
  %match_cmp418 = icmp eq i64 %ch370, 46
  br i1 %match_cmp418, label %match_arm_4, label %match_test_5

match_arm_4:                                      ; preds = %match_test_4
  %SpannedToken_init419 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init419, align 4
  %Token_ctor420 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor420, align 1
  %tag_ptr421 = getelementptr inbounds %Token, ptr %Token_ctor420, i32 0, i32 0
  store i8 68, ptr %tag_ptr421, align 1
  %enum_val422 = load %Token, ptr %Token_ctor420, align 1
  %SpannedToken.token.init423 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init419, i32 0, i32 0
  store %Token %enum_val422, ptr %SpannedToken.token.init423, align 1
  %sp424 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init425 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init419, i32 0, i32 1
  store %Span %sp424, ptr %SpannedToken.span.init425, align 4
  %SpannedToken_val426 = load %SpannedToken, ptr %SpannedToken_init419, align 4
  %l2427 = load %Lexer, ptr %l2363, align 8
  %tup0428 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val426, 0
  %tup1429 = insertvalue { %SpannedToken, %Lexer } %tup0428, %Lexer %l2427, 1
  ret { %SpannedToken, %Lexer } %tup1429

match_test_5:                                     ; preds = %match_test_4
  %match_cmp430 = icmp eq i64 %ch370, 62
  br i1 %match_cmp430, label %match_arm_5, label %match_test_6

match_arm_5:                                      ; preds = %match_test_5
  %SpannedToken_init431 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init431, align 4
  %Token_ctor432 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor432, align 1
  %tag_ptr433 = getelementptr inbounds %Token, ptr %Token_ctor432, i32 0, i32 0
  store i8 69, ptr %tag_ptr433, align 1
  %enum_val434 = load %Token, ptr %Token_ctor432, align 1
  %SpannedToken.token.init435 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init431, i32 0, i32 0
  store %Token %enum_val434, ptr %SpannedToken.token.init435, align 1
  %sp436 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init437 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init431, i32 0, i32 1
  store %Span %sp436, ptr %SpannedToken.span.init437, align 4
  %SpannedToken_val438 = load %SpannedToken, ptr %SpannedToken_init431, align 4
  %l2439 = load %Lexer, ptr %l2363, align 8
  %tup0440 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val438, 0
  %tup1441 = insertvalue { %SpannedToken, %Lexer } %tup0440, %Lexer %l2439, 1
  ret { %SpannedToken, %Lexer } %tup1441

match_test_6:                                     ; preds = %match_test_5
  %match_cmp442 = icmp eq i64 %ch370, 60
  br i1 %match_cmp442, label %match_arm_6, label %match_test_7

match_arm_6:                                      ; preds = %match_test_6
  %SpannedToken_init443 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init443, align 4
  %Token_ctor444 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor444, align 1
  %tag_ptr445 = getelementptr inbounds %Token, ptr %Token_ctor444, i32 0, i32 0
  store i8 70, ptr %tag_ptr445, align 1
  %enum_val446 = load %Token, ptr %Token_ctor444, align 1
  %SpannedToken.token.init447 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init443, i32 0, i32 0
  store %Token %enum_val446, ptr %SpannedToken.token.init447, align 1
  %sp448 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init449 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init443, i32 0, i32 1
  store %Span %sp448, ptr %SpannedToken.span.init449, align 4
  %SpannedToken_val450 = load %SpannedToken, ptr %SpannedToken_init443, align 4
  %l2451 = load %Lexer, ptr %l2363, align 8
  %tup0452 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val450, 0
  %tup1453 = insertvalue { %SpannedToken, %Lexer } %tup0452, %Lexer %l2451, 1
  ret { %SpannedToken, %Lexer } %tup1453

match_test_7:                                     ; preds = %match_test_6
  %match_cmp454 = icmp eq i64 %ch370, 61
  br i1 %match_cmp454, label %match_arm_7, label %match_test_8

match_arm_7:                                      ; preds = %match_test_7
  %SpannedToken_init455 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init455, align 4
  %Token_ctor456 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor456, align 1
  %tag_ptr457 = getelementptr inbounds %Token, ptr %Token_ctor456, i32 0, i32 0
  store i8 73, ptr %tag_ptr457, align 1
  %enum_val458 = load %Token, ptr %Token_ctor456, align 1
  %SpannedToken.token.init459 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init455, i32 0, i32 0
  store %Token %enum_val458, ptr %SpannedToken.token.init459, align 1
  %sp460 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init461 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init455, i32 0, i32 1
  store %Span %sp460, ptr %SpannedToken.span.init461, align 4
  %SpannedToken_val462 = load %SpannedToken, ptr %SpannedToken_init455, align 4
  %l2463 = load %Lexer, ptr %l2363, align 8
  %tup0464 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val462, 0
  %tup1465 = insertvalue { %SpannedToken, %Lexer } %tup0464, %Lexer %l2463, 1
  ret { %SpannedToken, %Lexer } %tup1465

match_test_8:                                     ; preds = %match_test_7
  %match_cmp466 = icmp eq i64 %ch370, 58
  br i1 %match_cmp466, label %match_arm_8, label %match_test_9

match_arm_8:                                      ; preds = %match_test_8
  %SpannedToken_init467 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init467, align 4
  %Token_ctor468 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor468, align 1
  %tag_ptr469 = getelementptr inbounds %Token, ptr %Token_ctor468, i32 0, i32 0
  store i8 76, ptr %tag_ptr469, align 1
  %enum_val470 = load %Token, ptr %Token_ctor468, align 1
  %SpannedToken.token.init471 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init467, i32 0, i32 0
  store %Token %enum_val470, ptr %SpannedToken.token.init471, align 1
  %sp472 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init473 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init467, i32 0, i32 1
  store %Span %sp472, ptr %SpannedToken.span.init473, align 4
  %SpannedToken_val474 = load %SpannedToken, ptr %SpannedToken_init467, align 4
  %l2475 = load %Lexer, ptr %l2363, align 8
  %tup0476 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val474, 0
  %tup1477 = insertvalue { %SpannedToken, %Lexer } %tup0476, %Lexer %l2475, 1
  ret { %SpannedToken, %Lexer } %tup1477

match_test_9:                                     ; preds = %match_test_8
  %match_cmp478 = icmp eq i64 %ch370, 59
  br i1 %match_cmp478, label %match_arm_9, label %match_test_10

match_arm_9:                                      ; preds = %match_test_9
  %SpannedToken_init479 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init479, align 4
  %Token_ctor480 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor480, align 1
  %tag_ptr481 = getelementptr inbounds %Token, ptr %Token_ctor480, i32 0, i32 0
  store i8 77, ptr %tag_ptr481, align 1
  %enum_val482 = load %Token, ptr %Token_ctor480, align 1
  %SpannedToken.token.init483 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init479, i32 0, i32 0
  store %Token %enum_val482, ptr %SpannedToken.token.init483, align 1
  %sp484 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init485 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init479, i32 0, i32 1
  store %Span %sp484, ptr %SpannedToken.span.init485, align 4
  %SpannedToken_val486 = load %SpannedToken, ptr %SpannedToken_init479, align 4
  %l2487 = load %Lexer, ptr %l2363, align 8
  %tup0488 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val486, 0
  %tup1489 = insertvalue { %SpannedToken, %Lexer } %tup0488, %Lexer %l2487, 1
  ret { %SpannedToken, %Lexer } %tup1489

match_test_10:                                    ; preds = %match_test_9
  %match_cmp490 = icmp eq i64 %ch370, 38
  br i1 %match_cmp490, label %match_arm_10, label %match_test_11

match_arm_10:                                     ; preds = %match_test_10
  %SpannedToken_init491 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init491, align 4
  %Token_ctor492 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor492, align 1
  %tag_ptr493 = getelementptr inbounds %Token, ptr %Token_ctor492, i32 0, i32 0
  store i8 78, ptr %tag_ptr493, align 1
  %enum_val494 = load %Token, ptr %Token_ctor492, align 1
  %SpannedToken.token.init495 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init491, i32 0, i32 0
  store %Token %enum_val494, ptr %SpannedToken.token.init495, align 1
  %sp496 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init497 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init491, i32 0, i32 1
  store %Span %sp496, ptr %SpannedToken.span.init497, align 4
  %SpannedToken_val498 = load %SpannedToken, ptr %SpannedToken_init491, align 4
  %l2499 = load %Lexer, ptr %l2363, align 8
  %tup0500 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val498, 0
  %tup1501 = insertvalue { %SpannedToken, %Lexer } %tup0500, %Lexer %l2499, 1
  ret { %SpannedToken, %Lexer } %tup1501

match_test_11:                                    ; preds = %match_test_10
  %match_cmp502 = icmp eq i64 %ch370, 124
  br i1 %match_cmp502, label %match_arm_11, label %match_test_12

match_arm_11:                                     ; preds = %match_test_11
  %SpannedToken_init503 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init503, align 4
  %Token_ctor504 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor504, align 1
  %tag_ptr505 = getelementptr inbounds %Token, ptr %Token_ctor504, i32 0, i32 0
  store i8 79, ptr %tag_ptr505, align 1
  %enum_val506 = load %Token, ptr %Token_ctor504, align 1
  %SpannedToken.token.init507 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init503, i32 0, i32 0
  store %Token %enum_val506, ptr %SpannedToken.token.init507, align 1
  %sp508 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init509 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init503, i32 0, i32 1
  store %Span %sp508, ptr %SpannedToken.span.init509, align 4
  %SpannedToken_val510 = load %SpannedToken, ptr %SpannedToken_init503, align 4
  %l2511 = load %Lexer, ptr %l2363, align 8
  %tup0512 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val510, 0
  %tup1513 = insertvalue { %SpannedToken, %Lexer } %tup0512, %Lexer %l2511, 1
  ret { %SpannedToken, %Lexer } %tup1513

match_test_12:                                    ; preds = %match_test_11
  %match_cmp514 = icmp eq i64 %ch370, 33
  br i1 %match_cmp514, label %match_arm_12, label %match_test_13

match_arm_12:                                     ; preds = %match_test_12
  %SpannedToken_init515 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init515, align 4
  %Token_ctor516 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor516, align 1
  %tag_ptr517 = getelementptr inbounds %Token, ptr %Token_ctor516, i32 0, i32 0
  store i8 80, ptr %tag_ptr517, align 1
  %enum_val518 = load %Token, ptr %Token_ctor516, align 1
  %SpannedToken.token.init519 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init515, i32 0, i32 0
  store %Token %enum_val518, ptr %SpannedToken.token.init519, align 1
  %sp520 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init521 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init515, i32 0, i32 1
  store %Span %sp520, ptr %SpannedToken.span.init521, align 4
  %SpannedToken_val522 = load %SpannedToken, ptr %SpannedToken_init515, align 4
  %l2523 = load %Lexer, ptr %l2363, align 8
  %tup0524 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val522, 0
  %tup1525 = insertvalue { %SpannedToken, %Lexer } %tup0524, %Lexer %l2523, 1
  ret { %SpannedToken, %Lexer } %tup1525

match_test_13:                                    ; preds = %match_test_12
  %match_cmp526 = icmp eq i64 %ch370, 63
  br i1 %match_cmp526, label %match_arm_13, label %match_test_14

match_arm_13:                                     ; preds = %match_test_13
  %SpannedToken_init527 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init527, align 4
  %Token_ctor528 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor528, align 1
  %tag_ptr529 = getelementptr inbounds %Token, ptr %Token_ctor528, i32 0, i32 0
  store i8 81, ptr %tag_ptr529, align 1
  %enum_val530 = load %Token, ptr %Token_ctor528, align 1
  %SpannedToken.token.init531 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init527, i32 0, i32 0
  store %Token %enum_val530, ptr %SpannedToken.token.init531, align 1
  %sp532 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init533 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init527, i32 0, i32 1
  store %Span %sp532, ptr %SpannedToken.span.init533, align 4
  %SpannedToken_val534 = load %SpannedToken, ptr %SpannedToken_init527, align 4
  %l2535 = load %Lexer, ptr %l2363, align 8
  %tup0536 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val534, 0
  %tup1537 = insertvalue { %SpannedToken, %Lexer } %tup0536, %Lexer %l2535, 1
  ret { %SpannedToken, %Lexer } %tup1537

match_test_14:                                    ; preds = %match_test_13
  %match_cmp538 = icmp eq i64 %ch370, 37
  br i1 %match_cmp538, label %match_arm_14, label %match_test_15

match_arm_14:                                     ; preds = %match_test_14
  %SpannedToken_init539 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init539, align 4
  %Token_ctor540 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor540, align 1
  %tag_ptr541 = getelementptr inbounds %Token, ptr %Token_ctor540, i32 0, i32 0
  store i8 82, ptr %tag_ptr541, align 1
  %enum_val542 = load %Token, ptr %Token_ctor540, align 1
  %SpannedToken.token.init543 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init539, i32 0, i32 0
  store %Token %enum_val542, ptr %SpannedToken.token.init543, align 1
  %sp544 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init545 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init539, i32 0, i32 1
  store %Span %sp544, ptr %SpannedToken.span.init545, align 4
  %SpannedToken_val546 = load %SpannedToken, ptr %SpannedToken_init539, align 4
  %l2547 = load %Lexer, ptr %l2363, align 8
  %tup0548 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val546, 0
  %tup1549 = insertvalue { %SpannedToken, %Lexer } %tup0548, %Lexer %l2547, 1
  ret { %SpannedToken, %Lexer } %tup1549

match_test_15:                                    ; preds = %match_test_14
  %match_cmp550 = icmp eq i64 %ch370, 123
  br i1 %match_cmp550, label %match_arm_15, label %match_test_16

match_arm_15:                                     ; preds = %match_test_15
  %SpannedToken_init551 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init551, align 4
  %Token_ctor552 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor552, align 1
  %tag_ptr553 = getelementptr inbounds %Token, ptr %Token_ctor552, i32 0, i32 0
  store i8 83, ptr %tag_ptr553, align 1
  %enum_val554 = load %Token, ptr %Token_ctor552, align 1
  %SpannedToken.token.init555 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init551, i32 0, i32 0
  store %Token %enum_val554, ptr %SpannedToken.token.init555, align 1
  %sp556 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init557 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init551, i32 0, i32 1
  store %Span %sp556, ptr %SpannedToken.span.init557, align 4
  %SpannedToken_val558 = load %SpannedToken, ptr %SpannedToken_init551, align 4
  %l2559 = load %Lexer, ptr %l2363, align 8
  %tup0560 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val558, 0
  %tup1561 = insertvalue { %SpannedToken, %Lexer } %tup0560, %Lexer %l2559, 1
  ret { %SpannedToken, %Lexer } %tup1561

match_test_16:                                    ; preds = %match_test_15
  %match_cmp562 = icmp eq i64 %ch370, 125
  br i1 %match_cmp562, label %match_arm_16, label %match_test_17

match_arm_16:                                     ; preds = %match_test_16
  %SpannedToken_init563 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init563, align 4
  %Token_ctor564 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor564, align 1
  %tag_ptr565 = getelementptr inbounds %Token, ptr %Token_ctor564, i32 0, i32 0
  store i8 84, ptr %tag_ptr565, align 1
  %enum_val566 = load %Token, ptr %Token_ctor564, align 1
  %SpannedToken.token.init567 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init563, i32 0, i32 0
  store %Token %enum_val566, ptr %SpannedToken.token.init567, align 1
  %sp568 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init569 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init563, i32 0, i32 1
  store %Span %sp568, ptr %SpannedToken.span.init569, align 4
  %SpannedToken_val570 = load %SpannedToken, ptr %SpannedToken_init563, align 4
  %l2571 = load %Lexer, ptr %l2363, align 8
  %tup0572 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val570, 0
  %tup1573 = insertvalue { %SpannedToken, %Lexer } %tup0572, %Lexer %l2571, 1
  ret { %SpannedToken, %Lexer } %tup1573

match_test_17:                                    ; preds = %match_test_16
  %match_cmp574 = icmp eq i64 %ch370, 91
  br i1 %match_cmp574, label %match_arm_17, label %match_test_18

match_arm_17:                                     ; preds = %match_test_17
  %SpannedToken_init575 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init575, align 4
  %Token_ctor576 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor576, align 1
  %tag_ptr577 = getelementptr inbounds %Token, ptr %Token_ctor576, i32 0, i32 0
  store i8 85, ptr %tag_ptr577, align 1
  %enum_val578 = load %Token, ptr %Token_ctor576, align 1
  %SpannedToken.token.init579 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init575, i32 0, i32 0
  store %Token %enum_val578, ptr %SpannedToken.token.init579, align 1
  %sp580 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init581 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init575, i32 0, i32 1
  store %Span %sp580, ptr %SpannedToken.span.init581, align 4
  %SpannedToken_val582 = load %SpannedToken, ptr %SpannedToken_init575, align 4
  %l2583 = load %Lexer, ptr %l2363, align 8
  %tup0584 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val582, 0
  %tup1585 = insertvalue { %SpannedToken, %Lexer } %tup0584, %Lexer %l2583, 1
  ret { %SpannedToken, %Lexer } %tup1585

match_test_18:                                    ; preds = %match_test_17
  %match_cmp586 = icmp eq i64 %ch370, 93
  br i1 %match_cmp586, label %match_arm_18, label %match_test_19

match_arm_18:                                     ; preds = %match_test_18
  %SpannedToken_init587 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init587, align 4
  %Token_ctor588 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor588, align 1
  %tag_ptr589 = getelementptr inbounds %Token, ptr %Token_ctor588, i32 0, i32 0
  store i8 86, ptr %tag_ptr589, align 1
  %enum_val590 = load %Token, ptr %Token_ctor588, align 1
  %SpannedToken.token.init591 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init587, i32 0, i32 0
  store %Token %enum_val590, ptr %SpannedToken.token.init591, align 1
  %sp592 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init593 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init587, i32 0, i32 1
  store %Span %sp592, ptr %SpannedToken.span.init593, align 4
  %SpannedToken_val594 = load %SpannedToken, ptr %SpannedToken_init587, align 4
  %l2595 = load %Lexer, ptr %l2363, align 8
  %tup0596 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val594, 0
  %tup1597 = insertvalue { %SpannedToken, %Lexer } %tup0596, %Lexer %l2595, 1
  ret { %SpannedToken, %Lexer } %tup1597

match_test_19:                                    ; preds = %match_test_18
  %match_cmp598 = icmp eq i64 %ch370, 40
  br i1 %match_cmp598, label %match_arm_19, label %match_test_20

match_arm_19:                                     ; preds = %match_test_19
  %SpannedToken_init599 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init599, align 4
  %Token_ctor600 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor600, align 1
  %tag_ptr601 = getelementptr inbounds %Token, ptr %Token_ctor600, i32 0, i32 0
  store i8 87, ptr %tag_ptr601, align 1
  %enum_val602 = load %Token, ptr %Token_ctor600, align 1
  %SpannedToken.token.init603 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init599, i32 0, i32 0
  store %Token %enum_val602, ptr %SpannedToken.token.init603, align 1
  %sp604 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init605 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init599, i32 0, i32 1
  store %Span %sp604, ptr %SpannedToken.span.init605, align 4
  %SpannedToken_val606 = load %SpannedToken, ptr %SpannedToken_init599, align 4
  %l2607 = load %Lexer, ptr %l2363, align 8
  %tup0608 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val606, 0
  %tup1609 = insertvalue { %SpannedToken, %Lexer } %tup0608, %Lexer %l2607, 1
  ret { %SpannedToken, %Lexer } %tup1609

match_test_20:                                    ; preds = %match_test_19
  %match_cmp610 = icmp eq i64 %ch370, 41
  br i1 %match_cmp610, label %match_arm_20, label %match_test_21

match_arm_20:                                     ; preds = %match_test_20
  %SpannedToken_init611 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init611, align 4
  %Token_ctor612 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor612, align 1
  %tag_ptr613 = getelementptr inbounds %Token, ptr %Token_ctor612, i32 0, i32 0
  store i8 88, ptr %tag_ptr613, align 1
  %enum_val614 = load %Token, ptr %Token_ctor612, align 1
  %SpannedToken.token.init615 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init611, i32 0, i32 0
  store %Token %enum_val614, ptr %SpannedToken.token.init615, align 1
  %sp616 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init617 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init611, i32 0, i32 1
  store %Span %sp616, ptr %SpannedToken.span.init617, align 4
  %SpannedToken_val618 = load %SpannedToken, ptr %SpannedToken_init611, align 4
  %l2619 = load %Lexer, ptr %l2363, align 8
  %tup0620 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val618, 0
  %tup1621 = insertvalue { %SpannedToken, %Lexer } %tup0620, %Lexer %l2619, 1
  ret { %SpannedToken, %Lexer } %tup1621

match_test_21:                                    ; preds = %match_test_20
  %match_cmp622 = icmp eq i64 %ch370, 44
  br i1 %match_cmp622, label %match_arm_21, label %match_test_22

match_arm_21:                                     ; preds = %match_test_21
  %SpannedToken_init623 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init623, align 4
  %Token_ctor624 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor624, align 1
  %tag_ptr625 = getelementptr inbounds %Token, ptr %Token_ctor624, i32 0, i32 0
  store i8 89, ptr %tag_ptr625, align 1
  %enum_val626 = load %Token, ptr %Token_ctor624, align 1
  %SpannedToken.token.init627 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init623, i32 0, i32 0
  store %Token %enum_val626, ptr %SpannedToken.token.init627, align 1
  %sp628 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init629 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init623, i32 0, i32 1
  store %Span %sp628, ptr %SpannedToken.span.init629, align 4
  %SpannedToken_val630 = load %SpannedToken, ptr %SpannedToken_init623, align 4
  %l2631 = load %Lexer, ptr %l2363, align 8
  %tup0632 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val630, 0
  %tup1633 = insertvalue { %SpannedToken, %Lexer } %tup0632, %Lexer %l2631, 1
  ret { %SpannedToken, %Lexer } %tup1633

match_test_22:                                    ; preds = %match_test_21
  br label %match_arm_22

match_arm_22:                                     ; preds = %match_test_22
  %SpannedToken_init634 = alloca %SpannedToken, align 8
  store %SpannedToken zeroinitializer, ptr %SpannedToken_init634, align 4
  %Token_ctor635 = alloca %Token, align 8
  store %Token zeroinitializer, ptr %Token_ctor635, align 1
  %tag_ptr636 = getelementptr inbounds %Token, ptr %Token_ctor635, i32 0, i32 0
  store i8 95, ptr %tag_ptr636, align 1
  %payload_ptr637 = getelementptr inbounds %Token, ptr %Token_ctor635, i32 0, i32 1
  store %NomString { ptr @str.87, i64 7 }, ptr %payload_ptr637, align 8
  %enum_val638 = load %Token, ptr %Token_ctor635, align 1
  %SpannedToken.token.init639 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init634, i32 0, i32 0
  store %Token %enum_val638, ptr %SpannedToken.token.init639, align 1
  %sp640 = load %Span, ptr %sp369, align 4
  %SpannedToken.span.init641 = getelementptr inbounds %SpannedToken, ptr %SpannedToken_init634, i32 0, i32 1
  store %Span %sp640, ptr %SpannedToken.span.init641, align 4
  %SpannedToken_val642 = load %SpannedToken, ptr %SpannedToken_init634, align 4
  %l2643 = load %Lexer, ptr %l2363, align 8
  %tup0644 = insertvalue { %SpannedToken, %Lexer } undef, %SpannedToken %SpannedToken_val642, 0
  %tup1645 = insertvalue { %SpannedToken, %Lexer } %tup0644, %Lexer %l2643, 1
  ret { %SpannedToken, %Lexer } %tup1645
}
