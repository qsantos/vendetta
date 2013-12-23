#ifndef U_MINE_H
#define U_MINE_H

typedef struct kindOf_mine kindOf_mine_t;

#include <wchar.h>

#include "material.h"

struct kindOf_mine
{
	int id;
	wchar_t* name;

	int material_id;
	kindOf_material_t* material;
};

#endif
