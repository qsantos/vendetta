#ifndef U_MATERIAL_H
#define U_MATERIAL_H

typedef struct kindOf_material kindOf_material_t;

#include <wchar.h>

struct kindOf_material
{
	wchar_t* name;
};

void kindOf_material_exit(kindOf_material_t* m);

#endif
