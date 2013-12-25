#ifndef U_ITEM_H
#define U_ITEM_H

typedef struct kindOf_item kindOf_item_t;

#include <wchar.h>

struct kindOf_item
{
	wchar_t* name;

	int skill;
};

void kindOf_item_init(kindOf_item_t* i);
void kindOf_item_exit(kindOf_item_t* i);

#endif
