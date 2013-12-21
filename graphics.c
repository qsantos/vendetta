#include "graphics.h"

#include <stdlib.h>

void graphics_init(graphics_t* g)
{
	sfVideoMode mode = {800, 600, 32};
	g->render = sfRenderWindow_create(mode, "Vendetta 0.2", sfResize | sfClose, NULL);
	if (g->render == NULL)
		exit(1);
}

void graphics_exit(graphics_t* g)
{
	sfRenderWindow_destroy(g->render);
}
