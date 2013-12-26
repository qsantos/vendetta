#ifndef U_ITEM_H
#define U_ITEM_H

typedef struct kindOf_item kindOf_item_t;

#include <wchar.h>

#include "universe.h"
#include "skill.h"

struct kindOf_item
{
	wchar_t* name;

	int skill;
	int category;

	float  bonus_special[N_SPECIAL_SKILLS];
	float* bonus_material;
	float* bonus_item;
};

void kindOf_item_init(kindOf_item_t* i, universe_t* u);
void kindOf_item_exit(kindOf_item_t* i);

#endif
