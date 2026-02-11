sim:
ifeq ($(OS),Windows_NT)
	cargo simwin
else
	UNAME_S := $(shell uname -s)
	ifeq ($(UNAME_S),Darwin)
		cargo simmac
	else
		cargo sim
	endif
endif
