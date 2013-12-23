#include "material_list.h"

#include "../util.h"

void material_list_init(material_list_t* l)
{
	l->length  = 0;
	l->mat_ids = NULL;
	l->amounts = NULL;
}

void material_list_exit(material_list_t* l)
{
	free(l->amounts);
	free(l->mat_ids);
}

void material_list_push(material_list_t* l, int id, float a)
{
	int n = l->length++;
	l->mat_ids = CREALLOC(l->mat_ids, int,   l->length);
	l->amounts = CREALLOC(l->amounts, float, l->length);
	l->mat_ids[n] = id;
	l->amounts[n] = a;
}
