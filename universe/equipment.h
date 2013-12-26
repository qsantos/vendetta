#ifndef U_EQUIPMENT_H
#define U_EQUIPMENT_H

typedef struct kindOf_slot kindOf_slot_t;

#include <wchar.h>

struct kindOf_slot
{
	wchar_t* name;

	int category;
};

void kindOf_slot_init(kindOf_slot_t* s);
void kindOf_slot_exit(kindOf_slot_t* s);

#endif
